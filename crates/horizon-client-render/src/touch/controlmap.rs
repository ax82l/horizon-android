//! Named control maps: lists of custom buttons saved under `controlmaps/`.
//! Files are either Horizon's own format or a PojavLauncher layout, which is
//! converted on load so a Pojav HUD can be used without rebuilding it.
use super::CustomButton;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const DEFAULT_NAME: &str = "default";
const FOLDER: &str = "controlmaps";
const MAX_FILE: u64 = 4 << 20;
const DEFAULT_MAP: &str = include_str!("default_controlmap.json");

#[derive(Serialize, Deserialize)]
struct MapFile {
    version: u32,
    buttons: Vec<CustomButton>,
}

/// The layout every profile starts with (and "Reset" returns to).
pub fn default_buttons() -> Vec<CustomButton> {
    serde_json::from_str::<MapFile>(DEFAULT_MAP)
        .map(|file| file.buttons)
        .unwrap_or_default()
}

/// A usable file name: 1 to 64 characters, no path separators or control
/// characters, not hidden. Imported files keep whatever name they came with.
pub fn clean_name(name: &str) -> Option<String> {
    let name = name.trim();
    let ok = !name.is_empty()
        && name.chars().count() <= 64
        && !name.starts_with('.')
        && !name.contains(['/', '\\', ':', '\0'])
        && !name.chars().any(char::is_control);
    ok.then(|| name.to_owned())
}

pub fn folder(home: &Path) -> PathBuf {
    home.join(FOLDER)
}

/// Saved map names, sorted, without the `.json` extension.
pub fn list(home: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(folder(home))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let stem = name.strip_suffix(".json")?;
            clean_name(stem)
        })
        .collect();
    names.sort_by_key(|n| n.to_lowercase());
    names.dedup();
    names
}

pub fn save(home: &Path, name: &str, buttons: &[CustomButton]) -> Result<String, String> {
    let name = clean_name(name).ok_or("Use 1 to 64 characters, without / \\ or :")?;
    let dir = folder(home);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let file = MapFile {
        version: 1,
        buttons: buttons.to_vec(),
    };
    let bytes = serde_json::to_vec_pretty(&file).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{name}.json"));
    let temp = dir.join(format!("{name}.json.tmp"));
    std::fs::write(&temp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&temp, path).map_err(|e| e.to_string())?;
    Ok(name)
}

pub fn delete(home: &Path, name: &str) -> Result<(), String> {
    let name = clean_name(name).ok_or("Invalid name")?;
    std::fs::remove_file(folder(home).join(format!("{name}.json"))).map_err(|e| e.to_string())
}

/// Loads a map in either format. `surface` (pixels) and `density` (px per dp)
/// place the buttons of a Pojav layout, which stores screen-relative formulas.
pub fn load(
    home: &Path,
    name: &str,
    surface: [f32; 2],
    density: f32,
) -> Result<Vec<CustomButton>, String> {
    let name = clean_name(name).ok_or("Invalid name")?;
    let path = folder(home).join(format!("{name}.json"));
    let meta = std::fs::metadata(&path).map_err(|e| e.to_string())?;
    if meta.len() > MAX_FILE {
        return Err("File is too large".into());
    }
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    parse(&text, surface, density)
}

pub fn parse(text: &str, surface: [f32; 2], density: f32) -> Result<Vec<CustomButton>, String> {
    let value: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if value.get("mControlDataList").is_some() {
        return from_pojav(&value, surface, density);
    }
    let file: MapFile = serde_json::from_value(value).map_err(|e| e.to_string())?;
    let mut buttons = file.buttons;
    buttons.truncate(super::custom::MAX_BUTTONS);
    for button in &mut buttons {
        button.sanitize();
    }
    Ok(buttons)
}

fn android_key(code: i64) -> Option<String> {
    const LETTERS: &str = "abcdefghijklmnopqrstuvwxyz";
    let name = |s: &str| Some(format!("key.keyboard.{s}"));
    match code {
        7..=16 => name(&(code - 7).to_string()),
        29..=54 => name(&LETTERS[(code - 29) as usize..(code - 28) as usize]),
        131..=142 => name(&format!("f{}", code - 130)),
        19 => name("up"),
        20 => name("down"),
        21 => name("left"),
        22 => name("right"),
        55 => name("comma"),
        56 => name("period"),
        57 => name("left.alt"),
        58 => name("right.alt"),
        59 => name("left.shift"),
        60 => name("right.shift"),
        61 => name("tab"),
        62 => name("space"),
        66 => name("enter"),
        67 => name("backspace"),
        68 => name("grave.accent"),
        69 => name("minus"),
        70 => name("equal"),
        71 => name("left.bracket"),
        72 => name("right.bracket"),
        73 => name("backslash"),
        74 => name("semicolon"),
        75 => name("apostrophe"),
        76 => name("slash"),
        92 => name("page.up"),
        93 => name("page.down"),
        111 => name("escape"),
        112 => name("delete"),
        113 => name("left.control"),
        114 => name("right.control"),
        115 => name("caps.lock"),
        122 => name("home"),
        123 => name("end"),
        124 => name("insert"),
        143 => name("num.lock"),
        // Pojav's special buttons.
        -1 => Some("special:keyboard".into()),
        -2 => Some("special:hide_buttons".into()),
        -3 => Some("action:key.attack".into()),
        -4 => Some("action:key.use".into()),
        -6 => Some("action:key.pickItem".into()),
        _ => None,
    }
}

struct Expr<'a> {
    text: &'a [u8],
    at: usize,
    density: f64,
}
impl Expr<'_> {
    fn skip(&mut self) {
        while self.text.get(self.at).is_some_and(u8::is_ascii_whitespace) {
            self.at += 1;
        }
    }
    fn eat(&mut self, c: u8) -> bool {
        self.skip();
        if self.text.get(self.at) == Some(&c) {
            self.at += 1;
            true
        } else {
            false
        }
    }
    fn sum(&mut self) -> Option<f64> {
        let mut v = self.product()?;
        loop {
            if self.eat(b'+') {
                v += self.product()?;
            } else if self.eat(b'-') {
                v -= self.product()?;
            } else {
                return Some(v);
            }
        }
    }
    fn product(&mut self) -> Option<f64> {
        let mut v = self.unary()?;
        loop {
            if self.eat(b'*') {
                v *= self.unary()?;
            } else if self.eat(b'/') {
                let d = self.unary()?;
                v = if d == 0.0 { 0.0 } else { v / d };
            } else {
                return Some(v);
            }
        }
    }
    fn unary(&mut self) -> Option<f64> {
        if self.eat(b'-') {
            return Some(-self.unary()?);
        }
        if self.eat(b'+') {
            return self.unary();
        }
        self.atom()
    }
    fn atom(&mut self) -> Option<f64> {
        self.skip();
        if self.eat(b'(') {
            let v = self.sum()?;
            return self.eat(b')').then_some(v);
        }
        let start = self.at;
        while self
            .text
            .get(self.at)
            .is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_'))
        {
            self.at += 1;
        }
        let word = std::str::from_utf8(&self.text[start..self.at]).ok()?;
        if word.is_empty() {
            return None;
        }
        if word == "px" {
            if !self.eat(b'(') {
                return None;
            }
            let v = self.sum()?;
            return self.eat(b')').then_some(v * self.density);
        }
        word.parse().ok()
    }
}

/// Evaluates a Pojav position formula such as
/// `0.8 * ${screen_width} - px(84.3 / 100.0 * ${preferred_scale}) - ${width}`.
fn evaluate(formula: &str, vars: &[(&str, f64)], density: f64) -> Option<f64> {
    let mut text = formula.to_owned();
    for (name, value) in vars {
        text = text.replace(&format!("${{{name}}}"), &format!("({value})"));
    }
    let mut parser = Expr {
        text: text.as_bytes(),
        at: 0,
        density,
    };
    let v = parser.sum()?;
    parser.skip();
    (parser.at == text.len() && v.is_finite()).then_some(v)
}

fn from_pojav(
    value: &serde_json::Value,
    surface: [f32; 2],
    density: f32,
) -> Result<Vec<CustomButton>, String> {
    let list = value["mControlDataList"]
        .as_array()
        .ok_or("Not a Pojav control layout")?;
    let (sw, sh) = (f64::from(surface[0].max(1.0)), f64::from(surface[1].max(1.0)));
    let density = f64::from(density.max(0.5));
    let scale = value["scaledAt"].as_f64().unwrap_or(100.0).max(1.0);
    let mut buttons = Vec::new();
    for item in list {
        let mut keys: Vec<String> = Vec::new();
        for code in item["keycodes"].as_array().into_iter().flatten() {
            if let Some(key) = code.as_i64().filter(|c| *c != 0).and_then(android_key)
                && !keys.contains(&key)
            {
                keys.push(key);
            }
        }
        keys.truncate(4);
        if keys.is_empty() {
            continue;
        }
        let width_dp = item["width"].as_f64().unwrap_or(50.0) * scale / 100.0;
        let height_dp = item["height"].as_f64().unwrap_or(50.0) * scale / 100.0;
        let (w, h) = (width_dp * density, height_dp * density);
        let vars = [
            ("screen_width", sw),
            ("screen_height", sh),
            ("width", w),
            ("height", h),
            ("right", sw - w),
            ("bottom", sh - h),
            ("left", 0.0),
            ("top", 0.0),
            ("margin", 2.0 * density),
            ("preferred_scale", scale),
        ];
        let x = item["dynamicX"]
            .as_str()
            .and_then(|f| evaluate(f, &vars, density));
        let y = item["dynamicY"]
            .as_str()
            .and_then(|f| evaluate(f, &vars, density));
        let (Some(x), Some(y)) = (x, y) else {
            continue;
        };
        let color = |name: &str, default: u32| {
            item[name]
                .as_i64()
                .map_or(default, |v| (v & 0xFFFF_FFFF) as u32)
        };
        let mut button = CustomButton {
            name: item["name"].as_str().unwrap_or_default().to_owned(),
            keys,
            center: [((x + w / 2.0) / sw) as f32, ((y + h / 2.0) / sh) as f32],
            width: width_dp as f32,
            height: height_dp as f32,
            opacity: item["opacity"].as_f64().unwrap_or(1.0) as f32,
            toggle: item["isToggle"].as_bool().unwrap_or(false),
            swipeable: item["isSwipeable"].as_bool().unwrap_or(false),
            pass_through: item["passThruEnabled"].as_bool().unwrap_or(false),
            show_in_game: item["displayInGame"].as_bool().unwrap_or(true),
            show_in_menu: item["displayInMenu"].as_bool().unwrap_or(true),
            background: color("bgColor", 0x4D00_0000),
            stroke_color: color("strokeColor", 0xFFFF_FFFF),
            stroke_width: item["strokeWidth"].as_f64().unwrap_or(0.0) as f32,
            corner_radius: item["cornerRadius"].as_f64().unwrap_or(0.0) as f32,
        };
        button.sanitize();
        buttons.push(button);
        if buttons.len() >= super::custom::MAX_BUTTONS {
            break;
        }
    }
    if buttons.is_empty() {
        return Err("The layout has no usable buttons".into());
    }
    Ok(buttons)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_are_plain_and_bounded() {
        assert_eq!(clean_name("  My HUD-2 ").as_deref(), Some("My HUD-2"));
        assert!(clean_name("").is_none());
        assert!(clean_name("../x").is_none());
        assert!(clean_name(".hidden").is_none());
        assert!(clean_name(&"a".repeat(65)).is_none());
        assert_eq!(clean_name("Pojav (1)").as_deref(), Some("Pojav (1)"));
        assert!(clean_name("a:b").is_none());
    }
    #[test]
    fn pojav_formulas_evaluate_with_px_and_variables() {
        let vars = [("screen_width", 2000.0), ("width", 100.0), ("margin", 5.0)];
        let v = evaluate(
            "0.5 * ${screen_width} - px(10.0 / 100.0 * 50) - ${width} - ${margin}",
            &vars,
            2.0,
        )
        .unwrap();
        assert!((v - (1000.0 - 10.0 - 100.0 - 5.0)).abs() < 1e-9);
        assert!(evaluate("1 +", &vars, 1.0).is_none());
        assert_eq!(evaluate("-(2 + 3) * 2", &vars, 1.0), Some(-10.0));
    }
    #[test]
    fn a_pojav_layout_converts_keys_sizes_and_flags() {
        let json = r#"{"mControlDataList":[{
            "name":"W","keycodes":[51,0,0,0],"width":50.0,"height":60.0,
            "dynamicX":"0.5 * ${screen_width} - ${width} / 2",
            "dynamicY":"0.5 * ${screen_height} - ${height} / 2",
            "isToggle":true,"isSwipeable":true,"passThruEnabled":true,
            "displayInGame":true,"displayInMenu":false,
            "bgColor":1291845632,"strokeColor":-1,"strokeWidth":10.0,"cornerRadius":100.0,
            "opacity":0.5},
            {"name":"hidden","keycodes":[-1,0,0,0],"width":1,"height":1,
             "dynamicX":"0","dynamicY":"0"}],
            "scaledAt":100.0,"version":9}"#;
        let buttons = parse(json, [2000.0, 1000.0], 2.0).unwrap();
        assert_eq!(buttons.len(), 1);
        let b = &buttons[0];
        assert_eq!(b.keys, ["key.keyboard.w"]);
        assert!((b.center[0] - 0.5).abs() < 1e-4 && (b.center[1] - 0.5).abs() < 1e-4);
        assert_eq!((b.width, b.height), (50.0, 60.0));
        assert!(b.toggle && b.swipeable && b.pass_through);
        assert!(b.show_in_game && !b.show_in_menu);
        assert_eq!(b.background, 1291845632);
        assert_eq!(b.stroke_color, 0xFFFF_FFFF);
    }
    #[test]
    fn pojav_special_buttons_map_to_horizon_tokens() {
        assert_eq!(android_key(-3).as_deref(), Some("action:key.attack"));
        assert_eq!(android_key(-2).as_deref(), Some("special:hide_buttons"));
        assert_eq!(android_key(131).as_deref(), Some("key.keyboard.f1"));
        assert_eq!(android_key(29).as_deref(), Some("key.keyboard.a"));
        assert_eq!(android_key(54).as_deref(), Some("key.keyboard.z"));
        assert_eq!(android_key(-1).as_deref(), Some("special:keyboard"));
        assert!(android_key(-5).is_none());
    }
    #[test]
    fn maps_save_list_load_and_delete() {
        let home = std::env::temp_dir().join(format!("horizon-maps-{}", std::process::id()));
        let buttons = default_buttons();
        assert!(!buttons.is_empty());
        assert_eq!(save(&home, "Mine", &buttons).unwrap(), "Mine");
        assert_eq!(list(&home), ["Mine"]);
        let back = load(&home, "Mine", [2000.0, 1000.0], 2.0).unwrap();
        assert_eq!(back.len(), buttons.len());
        delete(&home, "Mine").unwrap();
        assert!(list(&home).is_empty());
        assert!(save(&home, "../bad", &buttons).is_err());
        let _ = std::fs::remove_dir_all(home);
    }
}
