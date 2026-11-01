//! Per-profile touch settings. Buttons are fully custom: positions are fractions
//! of the surface, sizes are density-independent pixels (dp) like PojavLauncher.
//! `Mode` and `Control` remain only as inert names the rest of the client still
//! mentions; touch is always the aim-crosshair scheme and no built-in control exists.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    #[default]
    Crosshair,
    Tap,
    Dpad,
}
impl Mode {
    pub const ALL: [Self; 3] = [Self::Crosshair, Self::Tap, Self::Dpad];
    pub fn label(self) -> &'static str {
        match self {
            Self::Crosshair => "Joystick & aim crosshair",
            Self::Tap => "Joystick & tap to interact",
            Self::Dpad => "D-Pad & tap to interact",
        }
    }
    pub const SCHEMES: [(Self, bool, &str); 4] = [
        (Self::Crosshair, false, "Joystick & aim crosshair"),
        (Self::Tap, false, "Joystick & tap to interact"),
        (Self::Dpad, false, "D-Pad & tap to interact"),
        (Self::Dpad, true, "D-Pad & aim crosshair"),
    ];
    pub fn index(self) -> usize {
        match self {
            Self::Crosshair => 0,
            Self::Tap => 1,
            Self::Dpad => 2,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    Move,
    Sprint,
    Jump,
    Sneak,
    Attack,
    Use,
    Chat,
    Pause,
    Settings,
    Inventory,
    Debug,
    Perspective,
    Offhand,
    Shield,
}
impl Control {
    pub const ALL: [Self; 14] = [
        Self::Move,
        Self::Sprint,
        Self::Jump,
        Self::Sneak,
        Self::Attack,
        Self::Use,
        Self::Chat,
        Self::Pause,
        Self::Settings,
        Self::Inventory,
        Self::Debug,
        Self::Perspective,
        Self::Offhand,
        Self::Shield,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Move => "Movement",
            Self::Sprint => "Sprint",
            Self::Jump => "Jump / fly up",
            Self::Sneak => "Sneak / fly down",
            Self::Attack => "Attack / mine",
            Self::Use => "Use / place",
            Self::Chat => "Chat",
            Self::Pause => "Pause",
            Self::Settings => "Touch settings",
            Self::Inventory => "Inventory",
            Self::Debug => "Debug overlay (F3)",
            Self::Perspective => "Camera perspective (F5)",
            Self::Offhand => "Swap hands",
            Self::Shield => "Raise shield",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub mode: Mode,
    pub split_controls: bool,
    pub contextual_interact: bool,
    pub left_handed: bool,
    pub sprint_with_joystick: bool,
    pub swap_jump_sneak: bool,
    pub sensitivity: f32,
    pub sensitivity_horizontal: f32,
    pub sensitivity_vertical: f32,
    pub offhand_button: bool,
    pub shield_button: bool,
    pub circle_size: f32,
    /// Multiplies every custom button's size.
    pub button_scale: f32,
    pub custom_buttons: Vec<super::CustomButton>,
    /// Name of the control map the buttons were last loaded from or saved as.
    pub controlmap: String,
    /// 2 = the dp-based button model; older files get the default map.
    pub custom_version: u32,
    pub analog_movement: bool,
    pub sprint_button: bool,
    pub sprint_toggle: bool,
    pub inventory_controls: bool,
    pub virtual_mouse_sensitivity: f32,
    /// World gestures: tap to use or place, long press to attack or mine,
    /// tap the hotbar to select. Off leaves only turning the view.
    pub crosshair_gestures: bool,
    pub gyroscope: bool,
    pub gyro_sensitivity: f32,
    pub attack_indicator_left: bool,
    pub horizontal_margin: f32,
    pub vertical_margin: f32,
}
const CUSTOM_VERSION: u32 = 2;
impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: Mode::Crosshair,
            split_controls: false,
            contextual_interact: false,
            left_handed: false,
            sprint_with_joystick: false,
            swap_jump_sneak: false,
            sensitivity: 1.0,
            sensitivity_horizontal: 1.0,
            sensitivity_vertical: 1.0,
            offhand_button: false,
            shield_button: false,
            circle_size: 32.0,
            button_scale: 1.0,
            custom_buttons: super::controlmap::default_buttons(),
            controlmap: super::controlmap::DEFAULT_NAME.into(),
            custom_version: CUSTOM_VERSION,
            analog_movement: false,
            sprint_button: true,
            sprint_toggle: false,
            inventory_controls: false,
            virtual_mouse_sensitivity: 1.0,
            crosshair_gestures: true,
            gyroscope: false,
            gyro_sensitivity: 1.0,
            attack_indicator_left: true,
            horizontal_margin: 0.0,
            vertical_margin: 0.0,
        }
    }
}
impl Settings {
    pub fn load() -> Self {
        Self::load_from(std::path::Path::new("horizon-touch.json"))
    }
    pub fn load_from(path: &std::path::Path) -> Self {
        let mut s: Self = std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        if s.custom_version < CUSTOM_VERSION {
            // Buttons saved by the old square, three-scheme editor cannot be converted.
            s.custom_buttons = super::controlmap::default_buttons();
            s.controlmap = super::controlmap::DEFAULT_NAME.into();
            s.custom_version = CUSTOM_VERSION;
        }
        s.sanitize();
        s
    }
    pub fn sanitize(&mut self) {
        // One scheme only: aim with the crosshair, turn by dragging.
        self.mode = Mode::Crosshair;
        self.split_controls = false;
        self.virtual_mouse_sensitivity = if self.virtual_mouse_sensitivity.is_finite() {
            self.virtual_mouse_sensitivity.clamp(0.25, 3.0)
        } else {
            1.0
        };
        self.gyro_sensitivity = if self.gyro_sensitivity.is_finite() {
            self.gyro_sensitivity.clamp(0.1, 3.0)
        } else {
            1.0
        };
        for value in [&mut self.horizontal_margin, &mut self.vertical_margin] {
            *value = if value.is_finite() {
                value.clamp(0.0, 0.12)
            } else {
                0.0
            };
        }
        if !self.sensitivity.is_finite() {
            self.sensitivity = 1.0;
        }
        self.sensitivity = self.sensitivity.clamp(0.1, 3.0);
        for value in [
            &mut self.sensitivity_horizontal,
            &mut self.sensitivity_vertical,
        ] {
            *value = if value.is_finite() {
                value.clamp(0.1, 3.0)
            } else {
                1.0
            };
        }
        self.button_scale = if self.button_scale.is_finite() {
            self.button_scale.clamp(0.5, 2.0)
        } else {
            1.0
        };
        self.circle_size = if self.circle_size.is_finite() {
            self.circle_size.clamp(16.0, 64.0)
        } else {
            32.0
        };
        self.controlmap = super::controlmap::clean_name(&self.controlmap)
            .unwrap_or_else(|| super::controlmap::DEFAULT_NAME.into());
        self.custom_buttons.truncate(super::custom::MAX_BUTTONS);
        for button in &mut self.custom_buttons {
            button.sanitize();
        }
    }
    pub fn save(&self) -> std::io::Result<()> {
        self.save_to(std::path::Path::new("horizon-touch.json"))
    }
    pub fn save_to(&self, path: &std::path::Path) -> std::io::Result<()> {
        let temp = path.with_extension("json.tmp");
        std::fs::write(&temp, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(temp, path)
    }
    /// Touch is always the aim-crosshair scheme.
    pub fn crosshair(&self) -> bool {
        true
    }
}
pub fn contains([x, y, w, h]: [f32; 4], p: [f32; 2]) -> bool {
    (x..=x + w).contains(&p[0]) && (y..=y + h).contains(&p[1])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gyro_preferences_roundtrip_and_reject_bad_values() {
        let mut s = Settings::default();
        assert!(!s.gyroscope);
        s.gyroscope = true;
        s.gyro_sensitivity = 2.0;
        let restored: Settings = serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap();
        assert!(restored.gyroscope);
        assert_eq!(restored.gyro_sensitivity, 2.0);
        s.gyro_sensitivity = f32::NAN;
        s.sanitize();
        assert_eq!(s.gyro_sensitivity, 1.0);
    }
    #[test]
    fn defaults_ship_the_default_control_map_with_gestures_on() {
        let s = Settings::default();
        assert!(!s.custom_buttons.is_empty());
        assert!(s.crosshair_gestures);
        assert_eq!(s.controlmap, super::super::controlmap::DEFAULT_NAME);
    }
    #[test]
    fn files_from_the_old_editor_get_the_default_map_but_keep_preferences() {
        let dir = std::env::temp_dir().join(format!("horizon-touch-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("horizon-touch.json");
        std::fs::write(
            &path,
            br#"{"sensitivity":2.0,"custom_buttons":[{"key":"key.keyboard.z","layouts":[]}]}"#,
        )
        .unwrap();
        let s = Settings::load_from(&path);
        assert_eq!(s.sensitivity, 2.0);
        assert_eq!(
            s.custom_buttons.len(),
            super::super::controlmap::default_buttons().len()
        );
        let _ = std::fs::remove_dir_all(dir);
    }
    #[test]
    fn mode_is_always_the_crosshair_scheme() {
        let mut s = Settings::default();
        s.mode = Mode::Dpad;
        s.sanitize();
        assert_eq!(s.mode, Mode::Crosshair);
        assert!(s.crosshair());
    }
}
