//! Custom buttons in the style of PojavLauncher: any number of buttons, each
//! pressing up to four keys, with its own size in dp, colours and visibility.
use super::*;
use serde::{Deserialize, Serialize};

/// Most buttons one control map can hold.
pub(super) const MAX_BUTTONS: usize = 128;
/// Keys one button can press together.
pub(super) const MAX_KEYS: usize = 4;
const MAX_NAME_CHARS: usize = 16;
/// A key slot that presses nothing.
pub const UNKNOWN: &str = "key.keyboard.unknown";
/// Shows or hides every other button; the button holding it stays visible.
pub const HIDE_BUTTONS: &str = "special:hide_buttons";
/// Switches the virtual mouse in menus on and off.
pub const MOUSE_TOGGLE: &str = "special:mouse";
/// Opens the Android keyboard, to type into chat and other text fields.
pub const KEYBOARD: &str = "special:keyboard";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomButton {
    /// Label; empty shows the first key's name.
    pub name: String,
    /// One to four tokens: `key.keyboard.*`, `action:key.*`, `special:*` or the empty slot.
    pub keys: Vec<String>,
    /// Centre as a fraction of the screen.
    pub center: [f32; 2],
    /// Size in dp, like Pojav, scaled by the global button size.
    pub width: f32,
    pub height: f32,
    /// Whole-button transparency.
    pub opacity: f32,
    /// A tap switches the keys on, the next tap switches them off.
    pub toggle: bool,
    /// Sliding a finger onto another swipeable button moves the press there.
    pub swipeable: bool,
    /// The finger on the button also turns the view.
    pub pass_through: bool,
    pub show_in_game: bool,
    /// Shown over the inventory screens (when inventory controls are on).
    pub show_in_menu: bool,
    /// ARGB fill.
    pub background: u32,
    /// ARGB outline.
    pub stroke_color: u32,
    /// Outline thickness, 0 to 100 percent of half the button; 100 fills it.
    pub stroke_width: f32,
    /// 0 is square, 100 is a full circle.
    pub corner_radius: f32,
}
impl Default for CustomButton {
    fn default() -> Self {
        Self {
            name: String::new(),
            keys: vec![UNKNOWN.into()],
            center: [0.5, 0.5],
            width: 50.0,
            height: 50.0,
            opacity: 1.0,
            toggle: false,
            swipeable: false,
            pass_through: false,
            show_in_game: true,
            show_in_menu: true,
            background: 0x8000_0000,
            stroke_color: 0xFFFF_FFFF,
            stroke_width: 0.0,
            corner_radius: 0.0,
        }
    }
}
fn is_special(token: &str) -> bool {
    token.starts_with("special:")
}
impl CustomButton {
    /// A fresh button on the given key, offset so new ones do not stack.
    pub fn new(key: &str, index: usize) -> Self {
        Self {
            keys: vec![key.into()],
            center: [
                0.3 + (index % 5) as f32 * 0.08,
                0.3 + (index / 5 % 5) as f32 * 0.1,
            ],
            ..Self::default()
        }
    }
    pub fn has_key(&self, token: &str) -> bool {
        self.keys.iter().any(|k| k == token)
    }
    /// Tokens that actually press something.
    pub fn active_keys(&self) -> Vec<String> {
        self.keys
            .iter()
            .filter(|k| k.as_str() != UNKNOWN)
            .cloned()
            .collect()
    }
    pub(super) fn sanitize(&mut self) {
        self.keys.retain(|k| Self::valid_token(k));
        self.keys.truncate(MAX_KEYS);
        if self.keys.is_empty() {
            self.keys.push(UNKNOWN.into());
        }
        self.name = self.name.chars().take(MAX_NAME_CHARS).collect();
        let fix = |v: &mut f32, min: f32, max: f32, default: f32| {
            *v = if v.is_finite() {
                v.clamp(min, max)
            } else {
                default
            };
        };
        fix(&mut self.width, 8.0, 600.0, 50.0);
        fix(&mut self.height, 8.0, 600.0, 50.0);
        fix(&mut self.opacity, 0.0, 1.0, 1.0);
        fix(&mut self.stroke_width, 0.0, 100.0, 0.0);
        fix(&mut self.corner_radius, 0.0, 100.0, 0.0);
        for c in &mut self.center {
            fix(c, 0.0, 1.0, 0.5);
        }
    }
    pub fn valid_token(token: &str) -> bool {
        token == UNKNOWN
            || token == HIDE_BUTTONS
            || token == MOUSE_TOGGLE
            || token == KEYBOARD
            || crate::keybinds::key_code(token).is_some()
            || token
                .strip_prefix("action:")
                .is_some_and(|name| crate::keybinds::MAPPINGS.iter().any(|m| m.0 == name))
    }
    /// What a token is called in the editor and on a button.
    pub fn token_label(token: &str) -> String {
        match token {
            UNKNOWN => "None".into(),
            HIDE_BUTTONS => "Hide / show buttons".into(),
            MOUSE_TOGGLE => "Mouse on / off".into(),
            KEYBOARD => "Keyboard".into(),
            "action:key.attack" => "Mouse left (attack)".into(),
            "action:key.use" => "Mouse right (use)".into(),
            "action:key.pickItem" => "Mouse middle (pick)".into(),
            _ => {
                if let Some(name) = token.strip_prefix("action:") {
                    crate::text::Text::translate(name, &[]).plain()
                } else {
                    crate::keybinds::display_name(token).plain()
                }
            }
        }
    }
    /// The text on the button.
    pub fn label(&self) -> String {
        if !self.name.is_empty() {
            return self.name.clone();
        }
        match self.keys.first().map(String::as_str) {
            None | Some(UNKNOWN) => String::new(),
            Some(HIDE_BUTTONS) => "GUI".into(),
            Some(MOUSE_TOGGLE) => "Mouse".into(),
            Some(KEYBOARD) => "KB".into(),
            Some("action:key.attack") => "PRI".into(),
            Some("action:key.use") => "SEC".into(),
            Some("action:key.pickItem") => "MID".into(),
            Some(token) => Self::token_label(token),
        }
    }
}
/// Every choice the key picker offers: `(token, label)`.
pub(super) fn choices() -> Vec<(String, String)> {
    let mut all: Vec<(String, String)> = vec![
        (UNKNOWN.into(), CustomButton::token_label(UNKNOWN)),
        ("action:key.attack".into(), "Mouse left (attack)".into()),
        ("action:key.use".into(), "Mouse right (use)".into()),
        ("action:key.pickItem".into(), "Mouse middle (pick)".into()),
        (HIDE_BUTTONS.into(), CustomButton::token_label(HIDE_BUTTONS)),
        (MOUSE_TOGGLE.into(), CustomButton::token_label(MOUSE_TOGGLE)),
        (KEYBOARD.into(), CustomButton::token_label(KEYBOARD)),
    ];
    all.extend(crate::keybinds::available_keys().iter().map(|(name, _)| {
        (
            (*name).to_owned(),
            crate::keybinds::display_name(name).plain(),
        )
    }));
    all.extend(
        crate::keybinds::MAPPINGS
            .iter()
            .filter(|m| m.1 != UNKNOWN)
            .map(|(name, _, _)| {
                (
                    format!("action:{name}"),
                    format!("Action: {}", crate::text::Text::translate(name, &[]).plain()),
                )
            })
            .filter(|(token, _)| {
                !matches!(
                    token.as_str(),
                    "action:key.attack" | "action:key.use" | "action:key.pickItem"
                )
            }),
    );
    all
}

/// A finger that began on a button.
#[derive(Clone, Debug, Default)]
pub(super) struct Held {
    /// The button it is on; `None` once a swipe left every swipeable button.
    index: Option<usize>,
    /// Keys this finger keeps pressed (empty for a toggle tap).
    keys: Vec<String>,
    swipe: bool,
    /// The previous point of a pass-through finger, which also turns the view.
    pass: Option<[f32; 2]>,
}

impl TouchControls {
    fn px_per_dp(&self) -> f32 {
        let density = if self.density > 0.0 {
            self.density
        } else {
            2.75
        };
        density * self.settings.button_scale
    }
    pub(super) fn surface_px(&self) -> [f32; 2] {
        if self.surface[0] > 0.0 && self.surface[1] > 0.0 {
            self.surface
        } else {
            [1920.0, 1080.0]
        }
    }
    /// Button size as fractions of the screen.
    pub(super) fn custom_size(&self, index: usize) -> [f32; 2] {
        let b = &self.settings.custom_buttons[index];
        let [sw, sh] = self.surface_px();
        let k = self.px_per_dp();
        [b.width * k / sw, b.height * k / sh]
    }
    /// `[x, y, w, h]` as fractions of the screen.
    pub(super) fn custom_rect(&self, index: usize) -> [f32; 4] {
        let b = &self.settings.custom_buttons[index];
        let [w, h] = self.custom_size(index);
        [b.center[0] - w / 2.0, b.center[1] - h / 2.0, w, h]
    }
    /// Whether the button is drawn and touchable on the current screen.
    pub(super) fn custom_shown(&self, index: usize) -> bool {
        let button = &self.settings.custom_buttons[index];
        if self.editor.is_some() {
            return true;
        }
        if self.buttons_hidden && !button.has_key(HIDE_BUTTONS) {
            return false;
        }
        if self.inventory_mode {
            button.show_in_menu
        } else {
            button.show_in_game
        }
    }
    pub fn custom_at(&self, point: [f32; 2]) -> Option<usize> {
        (0..self.settings.custom_buttons.len())
            .rev()
            .find(|&i| self.custom_shown(i) && layout::contains(self.custom_rect(i), point))
    }
    pub fn control_at_point(&self, point: [f32; 2]) -> bool {
        self.custom_at(point).is_some()
    }
    pub fn auxiliary_at(&self, point: [f32; 2]) -> bool {
        self.custom_at(point).is_some()
    }
    /// Some finger or toggle button currently holds this token.
    fn holds(&self, token: &str) -> bool {
        self.held_custom_tokens().any(|held| held == token)
    }
    fn press_custom_keys(&mut self, keys: &[String]) {
        for key in keys {
            if self.holds(key) {
                continue;
            }
            match key.as_str() {
                HIDE_BUTTONS => self.buttons_hidden = !self.buttons_hidden,
                MOUSE_TOGGLE => self.mouse_toggle = true,
                KEYBOARD => self.keyboard_requested = true,
                _ if is_special(key) => {}
                _ => self.custom_events.push((key.clone(), true)),
            }
        }
    }
    fn release_custom_keys(&mut self, keys: Vec<String>) {
        for key in keys {
            if !self.holds(&key) && !is_special(&key) {
                self.custom_events.push((key, false));
            }
        }
    }
    /// Starts a finger on a button. A toggle button owns only the tap itself.
    fn custom_press(&mut self, id: u64, index: usize, point: [f32; 2]) {
        let button = &self.settings.custom_buttons[index];
        let keys = button.active_keys();
        let (toggle, swipe, pass) = (
            button.toggle,
            button.swipeable && !button.toggle,
            button.pass_through.then_some(point),
        );
        self.fingers.insert(id, (Role::Ignore, point));
        self.clicked = true;
        if toggle {
            self.custom_fingers.insert(
                id,
                Held {
                    index: Some(index),
                    keys: Vec::new(),
                    swipe: false,
                    pass,
                },
            );
            if let Some(held) = self.custom_toggled.remove(&index) {
                self.release_custom_keys(held);
            } else if !keys.is_empty() {
                self.press_custom_keys(&keys);
                self.custom_toggled.insert(index, keys);
            }
        } else {
            self.press_custom_keys(&keys);
            self.custom_fingers.insert(
                id,
                Held {
                    index: Some(index),
                    keys,
                    swipe,
                    pass,
                },
            );
        }
    }
    /// A swipeable finger left its button: press whichever swipeable button it
    /// is over now, or nothing until it reaches one.
    fn custom_swipe(&mut self, id: u64, point: [f32; 2], held: &Held) {
        let over = self
            .custom_at(point)
            .filter(|&i| {
                let b = &self.settings.custom_buttons[i];
                b.swipeable && !b.toggle
            });
        if over == held.index {
            return;
        }
        self.release_custom_keys(held.keys.clone());
        let keys = over
            .map(|i| self.settings.custom_buttons[i].active_keys())
            .unwrap_or_default();
        self.press_custom_keys(&keys);
        if let Some(entry) = self.custom_fingers.get_mut(&id) {
            entry.index = over;
            entry.keys = keys;
        }
    }
    /// Handles a touch that began on, or already belongs to, a button. Returns
    /// the look delta of a pass-through finger, or `None` when the touch is not
    /// a button's.
    pub(super) fn custom_update(
        &mut self,
        id: u64,
        point: [f32; 2],
        started: bool,
        ended: bool,
    ) -> Option<[f32; 2]> {
        if let Some(held) = self.custom_fingers.get(&id).cloned() {
            if ended {
                self.custom_fingers.remove(&id);
                self.fingers.remove(&id);
                self.release_custom_keys(held.keys);
                return Some([0.0; 2]);
            }
            let mut delta = [0.0; 2];
            if let Some(previous) = held.pass {
                delta = [point[0] - previous[0], point[1] - previous[1]];
                if let Some(entry) = self.custom_fingers.get_mut(&id) {
                    entry.pass = Some(point);
                }
            }
            if held.swipe {
                self.custom_swipe(id, point, &held);
            }
            return Some(delta);
        }
        if started && let Some(index) = self.custom_at(point) {
            self.custom_press(id, index, point);
            return Some([0.0; 2]);
        }
        None
    }
    /// Inventory overlays keep button holds; there is nothing else to press.
    pub fn inventory_update(
        &mut self,
        id: u64,
        point: [f32; 2],
        started: bool,
        ended: bool,
    ) -> bool {
        self.custom_update(id, point, started, ended).is_some()
    }
    pub fn held_custom_tokens(&self) -> impl Iterator<Item = &str> {
        self.custom_fingers
            .values()
            .flat_map(|held| held.keys.iter())
            .chain(self.custom_toggled.values().flatten())
            .map(String::as_str)
    }
    pub fn key_events(&mut self) -> Vec<(String, bool)> {
        std::mem::take(&mut self.custom_events)
    }
    pub(super) fn clear_custom(&mut self) {
        let mut keys: Vec<String> = std::mem::take(&mut self.custom_fingers)
            .into_values()
            .flat_map(|held| held.keys)
            .chain(
                std::mem::take(&mut self.custom_toggled)
                    .into_values()
                    .flatten(),
            )
            .filter(|k| !is_special(k))
            .collect();
        keys.sort();
        keys.dedup();
        self.custom_events
            .extend(keys.into_iter().map(|k| (k, false)));
    }
    pub(super) fn draw_custom(&self, b: &mut HudBatch, a: &HudAtlas) {
        let Some(white) = a.sprites.get("horizon/white") else {
            return;
        };
        let uv = [[(white.u0 + white.u1) / 2.0, (white.v0 + white.v1) / 2.0]; 4];
        let (sw, sh) = (b.width, b.height);
        for (i, button) in self.settings.custom_buttons.iter().enumerate() {
            if !self.custom_shown(i) {
                continue;
            }
            let [rx, ry, rw, rh] = self.custom_rect(i);
            let (x, y, w, h) = (rx * sw, ry * sh, rw * sw, rh * sh);
            let opacity = if self.editor.is_some() {
                button.opacity.max(0.3)
            } else {
                button.opacity
            };
            if opacity <= 0.0 {
                continue;
            }
            let pressed = self
                .held_custom_tokens()
                .any(|held| button.keys.iter().any(|k| k == held))
                || (button.has_key(HIDE_BUTTONS) && self.buttons_hidden);
            let half = w.min(h) / 2.0;
            let radius = button.corner_radius / 100.0 * half;
            let stroke = (button.stroke_width / 100.0 * half).min(half);
            let steps = if radius < 0.5 { 1 } else { 6 };
            let outer = rounded_path([x, y, w, h], radius, steps);
            let inner_rect = [x + stroke, y + stroke, w - 2.0 * stroke, h - 2.0 * stroke];
            let inner = rounded_path(inner_rect, (radius - stroke).max(0.0), steps);
            let center = [x + w / 2.0, y + h / 2.0];
            let fill = |argb: u32| tint(argb, opacity, pressed);
            if (button.background >> 24) > 0 {
                let path = if stroke > 0.0 { &inner } else { &outer };
                for pair in path.windows(2) {
                    b.quad([center, pair[0], pair[1], center], uv, fill(button.background));
                }
            }
            if stroke > 0.0 && (button.stroke_color >> 24) > 0 {
                for (o, n) in outer.windows(2).zip(inner.windows(2)) {
                    b.quad([o[0], o[1], n[1], n[0]], uv, tint(button.stroke_color, opacity, false));
                }
            }
            let label = button.label();
            if !label.is_empty() {
                let width = HudBatch::text_width(a, &label);
                let start = b.vertices.len();
                b.text(
                    a,
                    &label,
                    center[0] - width / 2.0,
                    center[1] - 4.0,
                    rgba(255, 255, 255, (opacity * 255.0).round() as u8),
                    true,
                );
                let scale = ((w - 4.0).max(1.0) / width.max(1.0))
                    .min(((h - 4.0).max(1.0)) / 8.0)
                    .min(2.0);
                b.scale_since(start, center[0], center[1], scale, scale);
            }
        }
    }
}
/// An ARGB colour as the HUD's packed colour, scaled by the button's opacity
/// and lightened while the button is held.
fn tint(argb: u32, opacity: f32, pressed: bool) -> u32 {
    let (mut r, mut g, mut bl) = (
        ((argb >> 16) & 0xff) as f32,
        ((argb >> 8) & 0xff) as f32,
        (argb & 0xff) as f32,
    );
    let mut alpha = (argb >> 24) as f32 / 255.0;
    if pressed {
        for c in [&mut r, &mut g, &mut bl] {
            *c += (255.0 - *c) * 0.4;
        }
        alpha = (alpha + 0.25).min(1.0);
    }
    rgba(
        r.round() as u8,
        g.round() as u8,
        bl.round() as u8,
        (alpha * opacity * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}
/// A rounded rectangle's outline, closed, with `steps` segments per corner.
fn rounded_path([x, y, w, h]: [f32; 4], radius: f32, steps: usize) -> Vec<[f32; 2]> {
    use std::f32::consts::PI;
    let r = radius.clamp(0.0, (w.min(h) / 2.0).max(0.0));
    let corners = [
        ([x + r, y + r], PI),
        ([x + w - r, y + r], 1.5 * PI),
        ([x + w - r, y + h - r], 0.0),
        ([x + r, y + h - r], 0.5 * PI),
    ];
    let mut points = Vec::with_capacity(4 * (steps + 1) + 1);
    for (c, start) in corners {
        for s in 0..=steps {
            let angle = start + 0.5 * PI * s as f32 / steps as f32;
            points.push([c[0] + r * angle.cos(), c[1] + r * angle.sin()]);
        }
    }
    points.push(points[0]);
    points
}

#[cfg(test)]
mod tests {
    use super::*;
    fn one_button(button: CustomButton) -> TouchControls {
        let mut t = TouchControls::default();
        t.viewport(1920, 1080);
        t.density = 2.0;
        t.settings.custom_buttons = vec![button];
        t
    }
    fn centre(t: &TouchControls, i: usize) -> [f32; 2] {
        t.settings.custom_buttons[i].center
    }
    #[test]
    fn size_is_in_dp_and_scales_with_density_and_button_size() {
        let mut button = CustomButton::new("key.keyboard.z", 0);
        button.width = 100.0;
        button.height = 50.0;
        let mut t = one_button(button);
        let [w, h] = t.custom_size(0);
        assert!((w - 200.0 / 1920.0).abs() < 1e-6);
        assert!((h - 100.0 / 1080.0).abs() < 1e-6);
        t.settings.button_scale = 2.0;
        assert!((t.custom_size(0)[0] - 400.0 / 1920.0).abs() < 1e-6);
    }
    #[test]
    fn macro_button_presses_every_key_and_releases_each_once() {
        let mut button = CustomButton::new("key.keyboard.w", 0);
        button.keys.push("key.keyboard.left.shift".into());
        let mut t = one_button(button);
        let p = centre(&t, 0);
        t.update(1, p, true, false);
        assert_eq!(
            t.key_events(),
            [
                ("key.keyboard.w".into(), true),
                ("key.keyboard.left.shift".into(), true)
            ]
        );
        t.update(1, p, false, true);
        assert_eq!(
            t.key_events(),
            [
                ("key.keyboard.w".into(), false),
                ("key.keyboard.left.shift".into(), false)
            ]
        );
    }
    #[test]
    fn empty_key_slots_press_nothing_and_the_touch_falls_through() {
        let mut t = one_button(CustomButton::new(UNKNOWN, 0));
        let p = centre(&t, 0);
        t.update(1, p, true, false);
        assert!(t.key_events().is_empty());
    }
    #[test]
    fn shared_keys_release_only_after_the_last_finger() {
        let mut t = one_button(CustomButton::new("key.keyboard.z", 0));
        let mut twin = CustomButton::new("key.keyboard.z", 0);
        twin.center = [0.9, 0.2];
        t.settings.custom_buttons.push(twin);
        let (a, b) = (centre(&t, 0), centre(&t, 1));
        t.update(1, a, true, false);
        t.update(2, b, true, false);
        assert_eq!(t.key_events(), [("key.keyboard.z".into(), true)]);
        t.update(1, a, false, true);
        assert!(t.key_events().is_empty());
        t.update(2, b, false, true);
        assert_eq!(t.key_events(), [("key.keyboard.z".into(), false)]);
    }
    #[test]
    fn toggle_button_holds_until_the_next_tap() {
        let mut button = CustomButton::new("key.keyboard.z", 0);
        button.toggle = true;
        let mut t = one_button(button);
        let p = centre(&t, 0);
        t.update(1, p, true, false);
        assert_eq!(t.key_events(), [("key.keyboard.z".into(), true)]);
        t.update(1, p, false, true);
        assert!(t.key_events().is_empty());
        assert_eq!(t.held_custom_tokens().count(), 1);
        t.update(2, p, true, false);
        assert_eq!(t.key_events(), [("key.keyboard.z".into(), false)]);
        t.update(2, p, false, true);
        assert_eq!(t.held_custom_tokens().count(), 0);
    }
    #[test]
    fn toggled_keys_release_when_controls_clear() {
        let mut button = CustomButton::new("key.keyboard.z", 0);
        button.toggle = true;
        let mut t = one_button(button);
        let p = centre(&t, 0);
        t.update(1, p, true, false);
        t.update(1, p, false, true);
        t.key_events();
        t.clear();
        assert_eq!(t.key_events(), [("key.keyboard.z".into(), false)]);
    }
    #[test]
    fn swiping_moves_the_press_between_swipeable_buttons() {
        let mut a = CustomButton::new("key.keyboard.a", 0);
        a.swipeable = true;
        a.center = [0.2, 0.5];
        let mut d = CustomButton::new("key.keyboard.d", 0);
        d.swipeable = true;
        d.center = [0.6, 0.5];
        let mut t = one_button(a);
        t.settings.custom_buttons.push(d);
        t.update(1, [0.2, 0.5], true, false);
        assert_eq!(t.key_events(), [("key.keyboard.a".into(), true)]);
        t.update(1, [0.6, 0.5], false, false);
        assert_eq!(
            t.key_events(),
            [
                ("key.keyboard.a".into(), false),
                ("key.keyboard.d".into(), true)
            ]
        );
        t.update(1, [0.4, 0.9], false, false);
        assert_eq!(t.key_events(), [("key.keyboard.d".into(), false)]);
        t.update(1, [0.2, 0.5], false, false);
        assert_eq!(t.key_events(), [("key.keyboard.a".into(), true)]);
        t.update(1, [0.2, 0.5], false, true);
        assert_eq!(t.key_events(), [("key.keyboard.a".into(), false)]);
    }
    #[test]
    fn plain_buttons_stay_pressed_when_the_finger_slides_off() {
        let mut t = one_button(CustomButton::new("key.keyboard.z", 0));
        let p = centre(&t, 0);
        t.update(1, p, true, false);
        t.key_events();
        t.update(1, [0.95, 0.95], false, false);
        assert!(t.key_events().is_empty());
        assert_eq!(t.held_custom_tokens().count(), 1);
    }
    #[test]
    fn pass_through_buttons_press_and_turn_the_view() {
        let mut button = CustomButton::new("key.keyboard.z", 0);
        button.pass_through = true;
        let mut t = one_button(button);
        let p = centre(&t, 0);
        assert_eq!(t.update(1, p, true, false), [0.0, 0.0]);
        assert_eq!(t.key_events(), [("key.keyboard.z".into(), true)]);
        let delta = t.update(1, [p[0] + 0.01, p[1] - 0.02], false, false);
        assert!((delta[0] - 0.01).abs() < 1e-6 && (delta[1] + 0.02).abs() < 1e-6);
        assert!(t.key_events().is_empty());
    }
    #[test]
    fn hide_button_hides_the_rest_but_never_itself() {
        let mut hide = CustomButton::new(HIDE_BUTTONS, 0);
        hide.center = [0.9, 0.1];
        let mut t = one_button(CustomButton::new("key.keyboard.z", 0));
        t.settings.custom_buttons.push(hide);
        let z = centre(&t, 0);
        assert!(t.custom_at(z).is_some());
        t.update(1, [0.9, 0.1], true, false);
        t.update(1, [0.9, 0.1], false, true);
        assert!(t.buttons_hidden);
        assert!(t.key_events().is_empty());
        assert!(t.custom_at(z).is_none());
        assert!(t.custom_at([0.9, 0.1]).is_some());
        t.update(2, [0.9, 0.1], true, false);
        t.update(2, [0.9, 0.1], false, true);
        assert!(!t.buttons_hidden);
        assert!(t.custom_at(z).is_some());
    }
    #[test]
    fn mouse_key_requests_a_toggle_without_sending_a_key() {
        let mut t = one_button(CustomButton::new(MOUSE_TOGGLE, 0));
        let p = centre(&t, 0);
        t.update(1, p, true, false);
        t.update(1, p, false, true);
        assert!(std::mem::take(&mut t.mouse_toggle));
        assert!(t.key_events().is_empty());
    }
    #[test]
    fn buttons_respect_game_and_menu_visibility() {
        let mut button = CustomButton::new("key.keyboard.z", 0);
        button.show_in_menu = false;
        let mut t = one_button(button);
        let p = centre(&t, 0);
        assert!(t.custom_at(p).is_some());
        t.inventory_mode = true;
        assert!(t.custom_at(p).is_none());
        t.inventory_mode = false;
        t.settings.custom_buttons[0].show_in_game = false;
        assert!(t.custom_at(p).is_none());
    }
    #[test]
    fn json_round_trips_and_missing_fields_take_defaults() {
        let mut b = CustomButton::new("key.keyboard.x", 3);
        b.keys.push("special:mouse".into());
        b.name = "Zoom".into();
        b.toggle = true;
        b.background = 0x4d00_0000;
        let back: CustomButton = serde_json::from_str(&serde_json::to_string(&b).unwrap()).unwrap();
        assert_eq!(back, b);
        let sparse: CustomButton = serde_json::from_str(r#"{"keys":["key.keyboard.q"]}"#).unwrap();
        assert!(sparse.show_in_game && sparse.show_in_menu && !sparse.toggle);
        assert_eq!(sparse.width, 50.0);
    }
    #[test]
    fn sanitize_limits_keys_name_ranges_and_button_count() {
        let mut s = Settings::default();
        s.custom_buttons.clear();
        for i in 0..(MAX_BUTTONS + 10) {
            s.custom_buttons
                .push(CustomButton::new("key.keyboard.z", i));
        }
        s.custom_buttons[0].keys = vec![
            "key.keyboard.a".into(),
            "not.a.key".into(),
            "key.keyboard.b".into(),
            "key.keyboard.c".into(),
            "key.keyboard.d".into(),
            "key.keyboard.e".into(),
        ];
        s.custom_buttons[0].name = "a very long button label indeed".into();
        s.custom_buttons[0].width = f32::NAN;
        s.custom_buttons[0].corner_radius = 400.0;
        s.custom_buttons[1].keys = vec!["bogus".into()];
        s.sanitize();
        assert_eq!(s.custom_buttons.len(), MAX_BUTTONS);
        assert_eq!(s.custom_buttons[0].keys.len(), MAX_KEYS);
        assert!(!s.custom_buttons[0].keys.iter().any(|k| k == "not.a.key"));
        assert_eq!(s.custom_buttons[0].name.chars().count(), MAX_NAME_CHARS);
        assert_eq!(s.custom_buttons[0].width, 50.0);
        assert_eq!(s.custom_buttons[0].corner_radius, 100.0);
        assert_eq!(s.custom_buttons[1].keys, [UNKNOWN]);
    }
    #[test]
    fn rounded_paths_are_closed_and_stay_in_their_rectangle() {
        let path = rounded_path([10.0, 20.0, 100.0, 60.0], 30.0, 6);
        assert_eq!(path.first(), path.last());
        for p in &path {
            assert!(p[0] >= 9.99 && p[0] <= 110.01 && p[1] >= 19.99 && p[1] <= 80.01);
        }
        let square = rounded_path([0.0, 0.0, 10.0, 10.0], 0.0, 1);
        assert!(square.contains(&[0.0, 0.0]) && square.contains(&[10.0, 10.0]));
    }
    #[test]
    fn labels_default_to_the_first_key() {
        assert_eq!(CustomButton::new("action:key.attack", 0).label(), "PRI");
        assert_eq!(CustomButton::new(UNKNOWN, 0).label(), "");
        let mut named = CustomButton::new("key.keyboard.w", 0);
        named.name = "Fwd".into();
        assert_eq!(named.label(), "Fwd");
    }
    #[test]
    fn tint_scales_alpha_by_opacity_and_lightens_when_held() {
        assert_eq!(tint(0xFF00_0000, 0.5, false), rgba(0, 0, 0, 128));
        assert_eq!(tint(0xFF00_0000, 1.0, true), rgba(102, 102, 102, 255));
        assert_eq!(tint(0x0000_0000, 1.0, false), rgba(0, 0, 0, 0));
    }
}
