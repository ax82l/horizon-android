//! User-added keys share drawing/hit geometry and retain per-finger ownership.
use super::*;
use layout::Placement;
use serde::{Deserialize, Serialize};
/// Most custom buttons one profile can hold.
pub(super) const MAX_BUTTONS: usize = 128;
/// Extra keys a macro button presses together with its main key.
const MAX_EXTRA_KEYS: usize = 3;
const MAX_NAME_CHARS: usize = 12;
fn yes() -> bool {
    true
}
fn is_true(v: &bool) -> bool {
    *v
}
fn is_false(v: &bool) -> bool {
    !*v
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomButton {
    pub key: String,
    pub layouts: [Placement; 3],
    /// Macro: more keys pressed and released together with `key`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_keys: Vec<String>,
    /// Label shown on the button; empty uses the key's name.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    /// A tap switches the keys on, the next tap switches them off.
    #[serde(default, skip_serializing_if = "is_false")]
    pub toggle: bool,
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub show_in_game: bool,
    /// Shown over the inventory screens (when inventory controls are on).
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub show_in_menu: bool,
    /// ARGB fill replacing the button texture.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<u32>,
    /// ARGB label colour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_color: Option<u32>,
}
impl CustomButton {
    pub fn new(key: String, index: usize) -> Self {
        Self {
            key,
            layouts: [Placement::new(
                0.68 + (index % 3) as f32 * 0.1,
                0.35 + (index / 3 % 3) as f32 * 0.12,
                0.105,
            ); 3],
            extra_keys: Vec::new(),
            name: String::new(),
            toggle: false,
            show_in_game: true,
            show_in_menu: true,
            background: None,
            text_color: None,
        }
    }
    /// Every key this button presses, main key first.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.key.as_str()).chain(self.extra_keys.iter().map(String::as_str))
    }
    pub(super) fn sanitize(&mut self) {
        self.extra_keys
            .retain(|k| k != "key.keyboard.unknown" && Self::valid_token(k));
        self.extra_keys.truncate(MAX_EXTRA_KEYS);
        self.name = self.name.chars().take(MAX_NAME_CHARS).collect();
    }
    pub fn valid_token(token: &str) -> bool {
        token == "key.keyboard.unknown"
            || crate::keybinds::key_code(token).is_some()
            || token
                .strip_prefix("action:")
                .is_some_and(|name| crate::keybinds::MAPPINGS.iter().any(|m| m.0 == name))
    }
    pub fn label(&self) -> String {
        if !self.name.is_empty() {
            return self.name.clone();
        }
        if self.key == "key.keyboard.unknown" {
            return "Button".into();
        }
        if let Some(name) = self.key.strip_prefix("action:") {
            return crate::text::Text::translate(name, &[]).plain();
        }
        crate::keybinds::display_name(&self.key).plain()
    }
}
pub(super) fn choices(raw_keys: bool) -> Vec<(String, String)> {
    if raw_keys {
        crate::keybinds::available_keys()
            .iter()
            .map(|(name, _)| ((*name).into(), crate::keybinds::display_name(name).plain()))
            .collect()
    } else {
        crate::keybinds::MAPPINGS
            .iter()
            .filter(|m| m.1 != "key.keyboard.unknown")
            .map(|(name, _, _)| {
                (
                    format!("action:{name}"),
                    crate::text::Text::translate(name, &[]).plain(),
                )
            })
            .collect()
    }
}
impl TouchControls {
    pub(super) fn inventory_token(control: Control) -> Option<&'static str> {
        match control {
            Control::Sneak => Some("key.keyboard.left.shift"),
            Control::Attack => Some("action:key.attack"),
            Control::Use => Some("action:key.use"),
            _ => None,
        }
    }
    pub fn inventory_update(
        &mut self,
        id: u64,
        point: [f32; 2],
        started: bool,
        ended: bool,
    ) -> bool {
        if self.custom_fingers.contains_key(&id) {
            return self.custom_update(id, point, false, ended);
        }
        if started
            && self.custom_at(point).is_none()
            && let Some(token) = self.control_at(point).and_then(Self::inventory_token)
        {
            let pressed = !self.holds(token);
            self.custom_fingers.insert(id, vec![token.into()]);
            self.fingers.insert(id, (Role::Ignore, point));
            if pressed {
                self.custom_events.push((token.into(), true));
            }
            return true;
        }
        false
    }
    pub(super) fn fit_placement(&self, mut p: Placement) -> Placement {
        p.size = (p.size * self.settings.button_scale)
            .min(self.aspect())
            .min(1.0);
        let half = [p.size / self.aspect() / 2.0, p.size / 2.0];
        for (i, half) in half.into_iter().enumerate() {
            p.center[i] = p.center[i].clamp(half, 1.0 - half);
        }
        p
    }
    pub(super) fn custom_placement(&self, index: usize) -> Placement {
        let mut p = self.settings.custom_buttons[index].layouts[self.settings.mode.index()];
        if self.settings.left_handed {
            p.center[0] = 1.0 - p.center[0];
        }
        self.fit_placement(p)
    }
    /// Whether the button is drawn and touchable on the current screen.
    pub(super) fn custom_shown(&self, index: usize) -> bool {
        let button = &self.settings.custom_buttons[index];
        self.editor.is_some()
            || if self.inventory_mode {
                button.show_in_menu
            } else {
                button.show_in_game
            }
    }
    pub fn custom_at(&self, point: [f32; 2]) -> Option<usize> {
        (0..self.settings.custom_buttons.len()).rev().find(|&i| {
            self.custom_shown(i)
                && layout::contains(self.custom_placement(i).rect(self.aspect()), point)
        })
    }
    /// Some finger or toggle button currently holds this key.
    fn holds(&self, token: &str) -> bool {
        self.held_custom_tokens().any(|held| held == token)
    }
    fn press_custom_keys(&mut self, keys: &[String]) {
        for key in keys {
            if !self.holds(key) {
                self.custom_events.push((key.clone(), true));
            }
        }
    }
    fn release_custom_keys(&mut self, keys: Vec<String>) {
        for key in keys {
            if !self.holds(&key) {
                self.custom_events.push((key, false));
            }
        }
    }
    pub(super) fn custom_update(
        &mut self,
        id: u64,
        point: [f32; 2],
        started: bool,
        ended: bool,
    ) -> bool {
        if let Some(keys) = self.custom_fingers.get(&id).cloned() {
            if ended {
                self.custom_fingers.remove(&id);
                self.fingers.remove(&id);
                self.release_custom_keys(keys);
            }
            return true;
        }
        if started && let Some(index) = self.custom_at(point) {
            let button = &self.settings.custom_buttons[index];
            let keys: Vec<String> = button
                .keys()
                .filter(|k| *k != "key.keyboard.unknown")
                .map(str::to_owned)
                .collect();
            let toggle = button.toggle;
            if keys.is_empty() {
                return false;
            }
            self.fingers.insert(id, (Role::Ignore, point));
            self.clicked = true;
            if toggle {
                // The finger only owns the tap; the keys stay held until the next one.
                self.custom_fingers.insert(id, Vec::new());
                if let Some(held) = self.custom_toggled.remove(&index) {
                    self.release_custom_keys(held);
                } else {
                    self.press_custom_keys(&keys);
                    self.custom_toggled.insert(index, keys);
                }
            } else {
                self.press_custom_keys(&keys);
                self.custom_fingers.insert(id, keys);
            }
            return true;
        }
        false
    }
    pub fn held_custom_tokens(&self) -> impl Iterator<Item = &str> {
        self.custom_fingers
            .values()
            .chain(self.custom_toggled.values())
            .flatten()
            .map(String::as_str)
    }
    pub fn key_events(&mut self) -> Vec<(String, bool)> {
        std::mem::take(&mut self.custom_events)
    }
    pub(super) fn clear_custom(&mut self) {
        let keys: std::collections::BTreeSet<_> = std::mem::take(&mut self.custom_fingers)
            .into_values()
            .chain(std::mem::take(&mut self.custom_toggled).into_values())
            .flatten()
            .collect();
        self.custom_events
            .extend(keys.into_iter().map(|k| (k, false)));
    }
    pub(super) fn draw_custom(&self, b: &mut HudBatch, a: &HudAtlas) {
        for (i, button) in self.settings.custom_buttons.iter().enumerate() {
            if !self.custom_shown(i) {
                continue;
            }
            let p = self.custom_placement(i);
            let size = p.size * b.height;
            let rect = [
                p.center[0] * b.width - size / 2.0,
                p.center[1] * b.height - size / 2.0,
                size,
                size,
            ];
            let pressed = self
                .held_custom_tokens()
                .any(|held| button.keys().any(|key| key == held));
            let opacity = if self.editor.is_some() {
                p.opacity.max(0.25)
            } else {
                p.opacity
            };
            if opacity <= 0.0 {
                continue;
            }
            if let Some(argb) = button.background {
                // Own colour: a plain fill. Its alpha scales with the button opacity.
                let alpha = ((argb >> 24) & 0xff) as f32 * opacity;
                let (r, g, bl) = if pressed {
                    (170, 210, 150)
                } else {
                    (
                        ((argb >> 16) & 0xff) as u8,
                        ((argb >> 8) & 0xff) as u8,
                        (argb & 0xff) as u8,
                    )
                };
                b.fill(
                    a,
                    rect[0],
                    rect[1],
                    rect[2],
                    rect[3],
                    rgba(r, g, bl, alpha.round() as u8),
                );
            } else {
                let tint = if pressed {
                    rgba(170, 210, 150, (opacity * 255.0) as u8)
                } else {
                    rgba(255, 255, 255, (opacity * 255.0) as u8)
                };
                if !theme_button(
                    b,
                    a,
                    "button",
                    rect,
                    pressed,
                    rgba(255, 255, 255, (opacity * 255.0) as u8),
                    true,
                ) {
                    nine(b, a, "ui/button_borderless_light", 2.0, rect, tint);
                }
            }
            let label = button.label();
            let width = HudBatch::text_width(a, &label);
            let start = b.vertices.len();
            let text = match (button.text_color, button.background) {
                (Some(argb), _) => rgba(
                    ((argb >> 16) & 0xff) as u8,
                    ((argb >> 8) & 0xff) as u8,
                    (argb & 0xff) as u8,
                    (((argb >> 24) & 0xff) as f32 * opacity).round() as u8,
                ),
                // Light texture takes dark text; an own (usually dark) fill takes white.
                (None, Some(_)) => rgba(255, 255, 255, (opacity * 255.0) as u8),
                (None, None) => rgba(25, 25, 25, (opacity * 255.0) as u8),
            };
            b.text(
                a,
                &label,
                rect[0] + (size - width) / 2.0,
                rect[1] + (size - 8.0) / 2.0,
                text,
                false,
            );
            let scale = ((size - 6.0).max(1.0) / width.max(1.0))
                .min(((size - 6.0).max(1.0) / 8.0).min(1.0));
            b.scale_since(
                start,
                p.center[0] * b.width,
                p.center[1] * b.height,
                scale,
                scale,
            );
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inventory_modifier_and_mouse_hold_independently_and_cancel_cleanly() {
        let mut t = TouchControls::default();
        t.viewport(1920, 1080);
        t.inventory_mode = true;
        t.settings.mode = Mode::Crosshair;
        for (id, control, token) in [
            (1, Control::Sneak, "key.keyboard.left.shift"),
            (2, Control::Attack, "action:key.attack"),
            (3, Control::Use, "action:key.use"),
        ] {
            assert!(t.visible(control));
            assert!(t.inventory_update(id, t.placement(control).center, true, false));
            assert_eq!(t.key_events(), [(token.into(), true)]);
        }
        t.clear_world_gesture();
        assert_eq!(t.held_custom_tokens().count(), 3);
        assert_eq!(t.poll_state(), PadState::default());
        assert!(t.inventory_update(2, [0.0, 0.0], false, true));
        assert_eq!(t.key_events(), [("action:key.attack".into(), false)]);
        t.clear();
        let released = t.key_events();
        assert!(released.contains(&("key.keyboard.left.shift".into(), false)));
        assert!(released.contains(&("action:key.use".into(), false)));
    }
    #[test]
    fn inventory_overlay_preserves_configured_visibility_and_placements() {
        let mut touch = TouchControls::default();
        for mode in Mode::ALL {
            touch.settings.mode = mode;
            touch.settings.button_visibility[Control::Attack as usize] = 2;
            let before: Vec<_> = Control::ALL
                .into_iter()
                .map(|c| (c, touch.visible(c), touch.placement(c).center))
                .collect();
            touch.inventory_mode = true;
            for (control, visible, center) in before {
                assert_eq!(touch.visible(control), visible);
                assert_eq!(touch.placement(control).center, center);
            }
            touch.inventory_mode = false;
        }
    }
    #[test]
    fn unassigned_button_emits_no_input() {
        let mut touch = TouchControls::default();
        touch.viewport(1920, 1080);
        touch
            .settings
            .custom_buttons
            .push(CustomButton::new("key.keyboard.unknown".into(), 0));
        let point = touch.custom_placement(0).center;
        touch.update(1, point, true, false);
        assert!(touch.key_events().is_empty());
    }
    #[test]
    fn custom_keys_hold_through_motion_and_shared_keys_release_only_after_last_finger() {
        let mut t = TouchControls::default();
        t.viewport(1920, 1080);
        t.settings
            .custom_buttons
            .push(CustomButton::new("key.keyboard.z".into(), 0));
        let p = t.custom_placement(0).center;
        t.update(1, p, true, false);
        t.update(2, p, true, false);
        assert_eq!(t.key_events(), [("key.keyboard.z".into(), true)]);
        t.update(1, [0.99, 0.1], false, false);
        assert!(t.key_events().is_empty());
        t.update(1, p, false, true);
        assert!(t.key_events().is_empty());
        t.update(2, p, false, true);
        assert_eq!(t.key_events(), [("key.keyboard.z".into(), false)]);
    }
    #[test]
    fn cancellation_releases_all_custom_keys_and_scaling_preserves_saved_sizes() {
        let mut t = TouchControls::default();
        t.viewport(1920, 1080);
        t.settings
            .custom_buttons
            .push(CustomButton::new("key.keyboard.z".into(), 0));
        let stored = t.settings.custom_buttons[0].layouts[0].size;
        t.settings.button_scale = 2.0;
        assert_eq!(t.custom_placement(0).size, stored * 2.0);
        t.update(1, t.custom_placement(0).center, true, false);
        t.key_events();
        t.clear();
        assert_eq!(t.key_events(), [("key.keyboard.z".into(), false)]);
        assert_eq!(t.settings.custom_buttons[0].layouts[0].size, stored);
    }

    fn one_button(button: CustomButton) -> TouchControls {
        let mut t = TouchControls::default();
        t.viewport(1920, 1080);
        t.settings.custom_buttons.push(button);
        t
    }
    #[test]
    fn macro_button_presses_every_key_and_releases_each_once() {
        let mut button = CustomButton::new("key.keyboard.w".into(), 0);
        button.extra_keys = vec!["key.keyboard.left.shift".into()];
        let mut t = one_button(button);
        let p = t.custom_placement(0).center;
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
    fn toggle_button_holds_until_the_next_tap() {
        let mut button = CustomButton::new("key.keyboard.z".into(), 0);
        button.toggle = true;
        let mut t = one_button(button);
        let p = t.custom_placement(0).center;
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
        let mut button = CustomButton::new("key.keyboard.z".into(), 0);
        button.toggle = true;
        let mut t = one_button(button);
        let p = t.custom_placement(0).center;
        t.update(1, p, true, false);
        t.update(1, p, false, true);
        t.key_events();
        t.clear();
        assert_eq!(t.key_events(), [("key.keyboard.z".into(), false)]);
    }
    #[test]
    fn buttons_respect_game_and_menu_visibility() {
        let mut button = CustomButton::new("key.keyboard.z".into(), 0);
        button.show_in_menu = false;
        let mut t = one_button(button);
        let p = t.custom_placement(0).center;
        assert!(t.custom_at(p).is_some());
        t.inventory_mode = true;
        assert!(t.custom_at(p).is_none());
        t.inventory_mode = false;
        t.settings.custom_buttons[0].show_in_game = false;
        assert!(t.custom_at(p).is_none());
    }
    #[test]
    fn old_button_json_loads_with_defaults_and_new_fields_round_trip() {
        let place = r#"{"center":[0.5,0.5],"size":0.1,"opacity":0.5}"#;
        let json = format!(r#"{{"key":"key.keyboard.z","layouts":[{place},{place},{place}]}}"#);
        let old: CustomButton = serde_json::from_str(&json).unwrap();
        assert!(old.show_in_game && old.show_in_menu && !old.toggle);
        assert!(old.extra_keys.is_empty() && old.name.is_empty() && old.background.is_none());
        let mut new = old.clone();
        new.extra_keys = vec!["key.keyboard.x".into()];
        new.name = "Zoom".into();
        new.toggle = true;
        new.background = Some(0x4d00_0000);
        let back: CustomButton = serde_json::from_str(&serde_json::to_string(&new).unwrap()).unwrap();
        assert_eq!(back.extra_keys, new.extra_keys);
        assert_eq!(back.name, "Zoom");
        assert!(back.toggle);
        assert_eq!(back.background, Some(0x4d00_0000));
    }
    #[test]
    fn sanitize_limits_macro_keys_name_and_button_count() {
        let mut s = Settings::default();
        for i in 0..(MAX_BUTTONS + 10) {
            s.custom_buttons
                .push(CustomButton::new("key.keyboard.z".into(), i));
        }
        s.custom_buttons[0].extra_keys = vec![
            "key.keyboard.a".into(),
            "not.a.key".into(),
            "key.keyboard.b".into(),
            "key.keyboard.c".into(),
            "key.keyboard.d".into(),
        ];
        s.custom_buttons[0].name = "a very long button label".into();
        s.sanitize();
        assert_eq!(s.custom_buttons.len(), MAX_BUTTONS);
        assert_eq!(s.custom_buttons[0].extra_keys.len(), 3);
        assert!(s.custom_buttons[0].extra_keys.iter().all(|k| k != "not.a.key"));
        assert_eq!(s.custom_buttons[0].name.chars().count(), 12);
    }
}
