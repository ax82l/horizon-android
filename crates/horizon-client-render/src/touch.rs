//! Touch input: PojavLauncher-style custom buttons plus world gestures.
//! The world is always aimed with the crosshair: drag to turn the view, tap to
//! use or place, hold to attack or mine. Everything else is a custom button.
use crate::gamepad::{Button, PadState};
use crate::hud::{HudAtlas, HudBatch, rgba};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    time::{Duration, Instant},
};
const HOLD_DELAY: Duration = Duration::from_millis(350);
mod controlmap;
mod custom;
mod editor;
pub use custom::CustomButton;
pub mod layout;
pub use layout::{Control, Mode, Settings};
/// Entity-placement items retain short-tap placement, like building blocks.
pub fn item_places_entity_on_tap(item: &str) -> bool {
    let Some(id) = item.strip_prefix("minecraft:") else {
        return false;
    };
    id.ends_with("_boat")
        || id.ends_with("_raft")
        || id.ends_with("_spawn_egg")
        || id == "minecart"
        || id.ends_with("_minecart")
        || matches!(
            id,
            "armor_stand" | "item_frame" | "glow_item_frame" | "painting" | "end_crystal"
        )
}

/// Surface-targeted actions should not wait for a long press.
pub fn item_uses_on_tap(item: &str) -> bool {
    item_places_entity_on_tap(item)
        || item
            .strip_prefix("minecraft:")
            .is_some_and(|id| id.ends_with("_bucket") && id != "milk_bucket")
        || matches!(
            item,
            "minecraft:bone_meal"
                | "minecraft:flint_and_steel"
                | "minecraft:fire_charge"
                | "minecraft:honeycomb"
                | "minecraft:glass_bottle"
                | "minecraft:bucket"
                | "minecraft:water_bucket"
                | "minecraft:lava_bucket"
                | "minecraft:powder_snow_bucket"
        )
}
pub fn projectile_crosshair(item: &str) -> bool {
    matches!(
        item,
        "minecraft:bow"
            | "minecraft:crossbow"
            | "minecraft:trident"
            | "minecraft:snowball"
            | "minecraft:egg"
            | "minecraft:ender_pearl"
            | "minecraft:experience_bottle"
            | "minecraft:splash_potion"
            | "minecraft:lingering_potion"
            | "minecraft:wind_charge"
            | "minecraft:fishing_rod"
    )
}
/// Equal physical drags turn equally on either axis, including portrait screens.
pub fn camera_delta(delta: [f32; 2], width: u32, height: u32, s: &Settings) -> [f32; 2] {
    let scale = width.max(height).max(1) as f32;
    [
        delta[0] * width as f32 / scale * 180.0 * s.sensitivity * s.sensitivity_horizontal,
        delta[1] * height as f32 / scale * 180.0 * s.sensitivity * s.sensitivity_vertical,
    ]
}

/// In tap modes the crosshair is absent, so use Java's native hotbar gauge.
pub fn attack_indicator(setting: u8, crosshair: bool) -> u8 {
    if setting == 1 && !crosshair {
        2
    } else {
        setting
    }
}

/// A finger ray through the perspective projection, without rotating the view.
pub fn screen_direction(
    forward: [f64; 3],
    up: [f64; 3],
    point: [f32; 2],
    aspect: f64,
    fov: f64,
) -> [f64; 3] {
    let right = [
        forward[1] * up[2] - forward[2] * up[1],
        forward[2] * up[0] - forward[0] * up[2],
        forward[0] * up[1] - forward[1] * up[0],
    ];
    let t = (fov.to_radians() / 2.0).tan();
    let x = (f64::from(point[0]) * 2.0 - 1.0) * aspect * t;
    let y = (1.0 - f64::from(point[1]) * 2.0) * t;
    let ray = std::array::from_fn::<_, 3, _>(|i| forward[i] + right[i] * x + up[i] * y);
    let length = ray.iter().map(|v| v * v).sum::<f64>().sqrt();
    ray.map(|v| v / length)
}

pub fn hotbar_slot(width: f32, height: f32, point: [f32; 2]) -> Option<usize> {
    let x = point[0] - (width / 2.0 - 90.0);
    (point[1] >= height - 22.0 && point[1] < height && (0.0..180.0).contains(&x))
        .then(|| (x / 20.0) as usize)
}
#[derive(Clone, Copy)]
enum Role {
    Look {
        start: [f32; 2],
        since: Instant,
        dragged: bool,
        held_use: bool,
        committed: bool,
    },
    Ignore,
}
#[derive(Default)]
pub struct TouchControls {
    pub settings: Settings,
    pub force_crosshair: bool,
    pub projectile_crosshair: bool,
    pub shield_available: bool,
    /// Physical keypad profiles keep utility controls without a movement overlay.
    pub auxiliary_only: bool,
    /// Inventory screens show the buttons marked "In menu".
    pub inventory_mode: bool,
    pub editor: Option<editor::Editor>,
    fingers: BTreeMap<u64, (Role, [f32; 2])>,
    /// Fingers that began on a custom button.
    custom_fingers: BTreeMap<u64, custom::Held>,
    /// Keys held on by toggle buttons, by button index.
    custom_toggled: BTreeMap<usize, Vec<String>>,
    custom_events: Vec<(String, bool)>,
    pending: PadState,
    pub aim: Option<[f32; 2]>,
    tap_aim: Option<[f32; 2]>,
    pending_tap: Option<bool>,
    aspect: f32,
    surface: [f32; 2],
    loaded: bool,
    /// Screen pixels per dp, set by the platform; button sizes are in dp.
    pub density: f32,
    /// The profile folder holding `controlmaps/`.
    pub home: PathBuf,
    /// "Hide / show buttons" is on: only the button holding that key shows.
    buttons_hidden: bool,
    /// A "Mouse on / off" button was pressed; the game takes and clears this.
    pub mouse_toggle: bool,
    /// A "Keyboard" button was pressed; the game shows the Android keyboard.
    pub keyboard_requested: bool,
    /// The editor wants a layout file picked; the game takes and clears this.
    pub import_requested: bool,
    pub chat_requested: bool,
    pub settings_requested: bool,
    pub debug_requested: bool,
    pub perspective_requested: bool,
    pub clicked: bool,
    pub context_use: bool,
    pub vertical_movement: bool,
    pub swimming: bool,
    pub mounted: bool,
    pub sneaking: bool,
    /// Snapshotted by a world gesture; drift cannot turn eating into mining.
    pub hold_uses_item: bool,
    pub holding_use: bool,
    pub holding: bool,
    pub break_progress: f32,
}
impl TouchControls {
    /// Touch always aims with the crosshair.
    pub fn crosshair(&self) -> bool {
        true
    }

    pub fn viewport(&mut self, width: u32, height: u32) {
        self.aspect = width.max(1) as f32 / height.max(1) as f32;
        self.surface = [width.max(1) as f32, height.max(1) as f32];
        if !self.loaded {
            self.settings = Settings::load();
            self.loaded = true;
        }
    }
    /// No built-in control exists any more; kept so callers need no change.
    pub fn control_at(&self, _point: [f32; 2]) -> Option<Control> {
        None
    }
    /// Inventory overlays retain custom key holds, but never apply world gestures.
    pub fn clear_world_gesture(&mut self) {
        self.fingers
            .retain(|_, (role, _)| matches!(role, Role::Ignore));
        self.pending = PadState::default();
        self.aim = None;
        self.tap_aim = None;
        self.pending_tap = None;
        self.holding = false;
        self.holding_use = false;
    }
    pub fn owns_pointer(&self, id: u64) -> bool {
        self.fingers.contains_key(&id) || self.custom_fingers.contains_key(&id)
    }
    pub fn ignore(&mut self, id: u64) {
        self.fingers.insert(id, (Role::Ignore, [0.0; 2]));
    }
    pub fn tap(&mut self, button: Button) {
        self.pending.set(button, true);
    }
    pub fn draw(&self, b: &mut HudBatch, a: &HudAtlas) {
        self.draw_custom(b, a);
        self.draw_editor(b, a);
    }
    pub fn shield_held(&self) -> bool {
        false
    }
    pub fn pointer(&self) -> Option<[f32; 2]> {
        None
    }
    /// Coordinates are fractions of the drawable surface. Returns a look delta.
    pub fn update(&mut self, id: u64, point: [f32; 2], started: bool, ended: bool) -> [f32; 2] {
        if let Some(delta) = self.custom_update(id, point, started, ended) {
            return delta;
        }
        if ended {
            if let Some((
                Role::Look {
                    since,
                    dragged,
                    held_use,
                    ..
                },
                _,
            )) = self.fingers.remove(&id)
                && self.settings.crosshair_gestures
                && !dragged
                && since.elapsed() < HOLD_DELAY
            {
                self.tap_aim = Some([0.5, 0.5]);
                self.pending_tap = Some(held_use);
            }
            return [0.0; 2];
        }
        if started {
            self.fingers.insert(
                id,
                (
                    Role::Look {
                        start: point,
                        since: Instant::now(),
                        dragged: false,
                        held_use: self.hold_uses_item,
                        committed: false,
                    },
                    point,
                ),
            );
            return [0.0; 2];
        }
        let aspect = if self.aspect > 0.0 {
            self.aspect
        } else {
            16.0 / 9.0
        };
        if let Some((role, previous)) = self.fingers.get_mut(&id) {
            let delta = if let Role::Look {
                start,
                dragged,
                since,
                committed,
                ..
            } = role
            {
                if !*dragged && since.elapsed() >= HOLD_DELAY {
                    *committed = true;
                }
                *dragged |= ((point[0] - start[0]) * aspect).hypot(point[1] - start[1]) > 0.018;
                if *dragged {
                    [point[0] - previous[0], point[1] - previous[1]]
                } else {
                    [0.0; 2]
                }
            } else {
                [0.0; 2]
            };
            *previous = point;
            return delta;
        }
        [0.0; 2]
    }
    pub fn clear(&mut self) {
        self.clear_custom();
        self.fingers.clear();
        self.pending = PadState::default();
        self.aim = None;
        self.tap_aim = None;
        self.pending_tap = None;
        self.holding_use = false;
        self.holding = false;
        self.chat_requested = false;
        self.settings_requested = false;
        self.debug_requested = false;
        self.perspective_requested = false;
    }
    /// A short world tap never holds a mouse button into the next tick.
    /// Its item-use intent is resolved against the current entity target.
    pub fn take_tap(&mut self) -> Option<bool> {
        self.pending_tap.take()
    }
    pub fn poll_state(&mut self) -> PadState {
        let mut state = std::mem::take(&mut self.pending);
        self.aim = self.tap_aim.take();
        self.holding_use = false;
        self.holding = false;
        if self.settings.crosshair_gestures {
            for (role, _) in self.fingers.values_mut() {
                if let Role::Look {
                    since,
                    dragged,
                    held_use,
                    committed,
                    ..
                } = role
                {
                    if !*dragged && since.elapsed() >= HOLD_DELAY {
                        *committed = true;
                    }
                    if !*dragged || *committed {
                        self.aim = Some([0.5, 0.5]);
                        if *committed {
                            self.holding = true;
                            self.holding_use = *held_use;
                            state.triggers[if *held_use { 0 } else { 1 }] = 1.0;
                            break;
                        }
                    }
                }
            }
        }
        state
    }
    /// Keep a contextual hold as use even after feeding consumed its last item.
    pub fn latch_use(&mut self) {
        for (role, _) in self.fingers.values_mut() {
            if let Role::Look {
                held_use,
                committed: true,
                ..
            } = role
            {
                *held_use = true;
                break;
            }
        }
        self.holding_use = true;
    }
}

/// Original optional resource-pack artwork. Missing states keep the default UI.
#[allow(clippy::too_many_arguments)]
pub fn theme_button(
    b: &mut HudBatch,
    a: &HudAtlas,
    name: &str,
    bounds: [f32; 4],
    pressed: bool,
    color: u32,
    frame: bool,
) -> bool {
    let (base, active) = match name {
        "chat" => (
            "gui/horizon/touch/chat_normal",
            "gui/horizon/touch/chat_pressed",
        ),
        "pause" => (
            "gui/horizon/touch/pause_normal",
            "gui/horizon/touch/pause_pressed",
        ),
        "settings" => (
            "gui/horizon/touch/settings_normal",
            "gui/horizon/touch/settings_pressed",
        ),
        "debug" => (
            "gui/horizon/touch/debug_normal",
            "gui/horizon/touch/debug_pressed",
        ),
        "perspective" => (
            "gui/horizon/touch/perspective_normal",
            "gui/horizon/touch/perspective_pressed",
        ),
        "button" => (
            "gui/horizon/touch/button_normal",
            "gui/horizon/touch/button_pressed",
        ),
        _ => return false,
    };
    let key = if pressed && a.sprites.contains_key(active) {
        active
    } else {
        base
    };
    let Some(rect) = a.sprites.get(key).copied() else {
        return false;
    };
    let [x, y, w, h] = bounds;
    if frame {
        crate::ui::nine_slice(b, a, key, x, y, w, h, color);
    } else {
        control_quad(b, a, x, y, w, h, rect, color);
    }
    true
}

fn nine(
    b: &mut HudBatch,
    a: &HudAtlas,
    name: &str,
    border: f32,
    [x, y, w, h]: [f32; 4],
    color: u32,
) {
    if let Some(r) = a.sprites.get(&format!("gui/bedrock/{name}")).copied() {
        let xs = [0.0, border, r.w as f32 - border, r.w as f32];
        let ys = [0.0, border, r.h as f32 - border, r.h as f32];
        let dx = [x, x + border, x + w - border, x + w];
        let dy = [y, y + border, y + h - border, y + h];
        for row in 0..3 {
            for col in 0..3 {
                let mut cell = r;
                cell.u0 = r.u0 + (r.u1 - r.u0) * xs[col] / r.w as f32;
                cell.u1 = r.u0 + (r.u1 - r.u0) * xs[col + 1] / r.w as f32;
                cell.v0 = r.v0 + (r.v1 - r.v0) * ys[row] / r.h as f32;
                cell.v1 = r.v0 + (r.v1 - r.v0) * ys[row + 1] / r.h as f32;
                control_quad(
                    b,
                    a,
                    dx[col],
                    dy[row],
                    dx[col + 1] - dx[col],
                    dy[row + 1] - dy[row],
                    cell,
                    color,
                );
            }
        }
    }
}

/// Atlas-wide clamp cannot protect an individual button from adjacent texels.
/// Keep UVs at texel centres and snap both screen edges to physical pixels.
#[allow(clippy::too_many_arguments)]
fn control_quad(
    b: &mut HudBatch,
    a: &HudAtlas,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    mut r: crate::hud::Rect,
    color: u32,
) {
    let half = [0.5 / a.width.max(1) as f32, 0.5 / a.height.max(1) as f32];
    r.u0 += half[0];
    r.u1 -= half[0];
    r.v0 += half[1];
    r.v1 -= half[1];
    let snap = |v: f32| (v * b.scale).round() / b.scale;
    let (left, top, right, bottom) = (snap(x), snap(y), snap(x + w), snap(y + h));
    b.rect_uv(left, top, right - left, bottom - top, r, color);
}

fn line(
    b: &mut crate::hud::HudBatch,
    a: &crate::hud::HudAtlas,
    p: [f32; 2],
    q: [f32; 2],
    color: u32,
) {
    let steps = (q[0] - p[0]).abs().max((q[1] - p[1]).abs()).ceil().max(1.0) as u32;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        b.fill(
            a,
            p[0] + (q[0] - p[0]) * t - 0.7,
            p[1] + (q[1] - p[1]) * t - 0.7,
            1.4,
            1.4,
            color,
        );
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn world() -> TouchControls {
        let mut t = TouchControls::default();
        t.viewport(1920, 1080);
        t.density = 2.0;
        t.settings.custom_buttons.clear();
        t
    }
    #[test]
    fn a_short_world_tap_uses_at_the_crosshair() {
        let mut t = world();
        t.update(1, [0.3, 0.4], true, false);
        t.update(1, [0.3, 0.4], false, true);
        assert_eq!(t.take_tap(), Some(false));
        let _ = t.poll_state();
        assert_eq!(t.aim, Some([0.5, 0.5]));
    }
    #[test]
    fn a_world_tap_can_carry_the_held_use_intent() {
        let mut t = world();
        t.hold_uses_item = true;
        t.update(1, [0.3, 0.4], true, false);
        t.update(1, [0.3, 0.4], false, true);
        assert_eq!(t.take_tap(), Some(true));
    }
    #[test]
    fn dragging_turns_the_view_and_never_taps() {
        let mut t = world();
        t.update(1, [0.3, 0.4], true, false);
        let delta = t.update(1, [0.4, 0.4], false, false);
        assert!((delta[0] - 0.1).abs() < 1e-6 && delta[1] == 0.0);
        t.update(1, [0.4, 0.4], false, true);
        assert_eq!(t.take_tap(), None);
    }
    #[test]
    fn holding_still_attacks_and_a_held_use_item_uses() {
        let mut t = world();
        t.update(1, [0.3, 0.4], true, false);
        if let Some((Role::Look { since, .. }, _)) = t.fingers.get_mut(&1) {
            *since = Instant::now() - HOLD_DELAY * 2;
        }
        let state = t.poll_state();
        assert_eq!(state.triggers[1], 1.0);
        assert!(t.holding && !t.holding_use);
        t.latch_use();
        let state = t.poll_state();
        assert_eq!(state.triggers[0], 1.0);
        assert!(t.holding_use);
    }
    #[test]
    fn disabling_gestures_leaves_only_turning() {
        let mut t = world();
        t.settings.crosshair_gestures = false;
        t.update(1, [0.3, 0.4], true, false);
        if let Some((Role::Look { since, .. }, _)) = t.fingers.get_mut(&1) {
            *since = Instant::now() - HOLD_DELAY * 2;
        }
        let state = t.poll_state();
        assert_eq!(state.triggers, [0.0, 0.0]);
        assert!(t.aim.is_none() && !t.holding);
        let delta = t.update(1, [0.5, 0.4], false, false);
        assert!(delta[0] > 0.0);
        t.update(1, [0.5, 0.4], false, true);
        assert_eq!(t.take_tap(), None);
    }
    #[test]
    fn buttons_take_the_touch_before_the_world() {
        let mut t = world();
        let button = CustomButton::new("key.keyboard.z", 0);
        let p = button.center;
        t.settings.custom_buttons.push(button);
        t.update(1, p, true, false);
        assert!(t.owns_pointer(1));
        t.update(1, p, false, true);
        assert_eq!(t.take_tap(), None);
        assert_eq!(t.key_events().len(), 2);
    }
    #[test]
    fn horizontal_and_vertical_drags_turn_equal_angles_for_equal_distances() {
        let s = Settings::default();
        let [h, v] = camera_delta([0.1, 0.1 * 1920.0 / 1080.0], 1920, 1080, &s);
        assert!((h - v).abs() < 1e-3);
    }
    #[test]
    fn hotbar_slots_map_from_the_centred_bar() {
        assert_eq!(hotbar_slot(400.0, 240.0, [200.0, 235.0]), Some(4));
        assert_eq!(hotbar_slot(400.0, 240.0, [10.0, 235.0]), None);
        assert_eq!(hotbar_slot(400.0, 240.0, [200.0, 100.0]), None);
    }
}
