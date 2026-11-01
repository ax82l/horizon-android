//! The control editor: tap a button to edit it, drag to move it. Everything is
//! drawn by the engine, so text is typed on a small keyboard drawn here.
use super::custom::{HIDE_BUTTONS, KEYBOARD, MAX_BUTTONS, MAX_KEYS, MOUSE_TOGGLE, UNKNOWN, choices};
use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tab {
    Basic,
    Behavior,
    Style,
}
const TABS: [(Tab, &str); 3] = [
    (Tab::Basic, "Basic"),
    (Tab::Behavior, "Behavior"),
    (Tab::Style, "Style"),
];
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Which {
    Background,
    Stroke,
}
impl Which {
    fn title(self) -> &'static str {
        match self {
            Self::Background => "Background color",
            Self::Stroke => "Stroke color",
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Row {
    Name,
    Width,
    Height,
    Key(usize),
    CloneDelete,
    Toggle,
    Swipe,
    PassThrough,
    InGame,
    InMenu,
    Opacity,
    Color(Which),
    StrokeWidth,
    CornerRadius,
}
fn rows(tab: Tab) -> Vec<Row> {
    match tab {
        Tab::Basic => vec![
            Row::Name,
            Row::Width,
            Row::Height,
            Row::Key(0),
            Row::Key(1),
            Row::Key(2),
            Row::Key(3),
            Row::CloneDelete,
        ],
        Tab::Behavior => vec![
            Row::Toggle,
            Row::Swipe,
            Row::PassThrough,
            Row::InGame,
            Row::InMenu,
            Row::Opacity,
        ],
        Tab::Style => vec![
            Row::Color(Which::Background),
            Row::Color(Which::Stroke),
            Row::StrokeWidth,
            Row::CornerRadius,
        ],
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Cat {
    Special,
    Letters,
    Numbers,
    Keys,
    Actions,
}
const CATS: [(Cat, &str); 5] = [
    (Cat::Special, "Special"),
    (Cat::Letters, "A-Z"),
    (Cat::Numbers, "0-9 F"),
    (Cat::Keys, "Keys"),
    (Cat::Actions, "Actions"),
];
fn category(token: &str) -> Cat {
    if token == UNKNOWN
        || token == HIDE_BUTTONS
        || token == MOUSE_TOGGLE
        || token == KEYBOARD
        || matches!(
            token,
            "action:key.attack" | "action:key.use" | "action:key.pickItem"
        )
    {
        return Cat::Special;
    }
    if token.starts_with("action:") {
        return Cat::Actions;
    }
    let name = token.strip_prefix("key.keyboard.").unwrap_or(token);
    if name.len() == 1 && name.as_bytes()[0].is_ascii_lowercase() {
        Cat::Letters
    } else if name.len() == 1
        || (name.starts_with('f') && name[1..].chars().all(|c| c.is_ascii_digit()))
    {
        Cat::Numbers
    } else {
        Cat::Keys
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PadMode {
    Alpha,
    Number,
    Hex,
}
#[derive(Clone, PartialEq, Eq, Debug)]
enum Target {
    Name,
    Width,
    Height,
    Hex(Which),
    Channel(Which, usize),
    MapName,
}
#[derive(Clone, Debug)]
struct TextInput {
    target: Target,
    mode: PadMode,
    buffer: String,
    shift: bool,
    title: String,
}
#[derive(Clone, Debug)]
enum Overlay {
    None,
    Keys { slot: usize, cat: Cat, page: usize },
    Text(TextInput),
    Color(Which),
    Maps { page: usize, confirm: Option<usize> },
    Options,
}
#[derive(Clone, Copy, PartialEq, Debug)]
enum Slider {
    Opacity,
    StrokeWidth,
    CornerRadius,
    Channel(Which, usize),
    ButtonScale,
}
#[derive(Clone, Copy, PartialEq, Debug)]
enum Drag {
    Button([f32; 2]),
    Slider(Slider, [f32; 4]),
}
pub struct Editor {
    original: Settings,
    pub selected: Option<usize>,
    tab: Tab,
    overlay: Overlay,
    drag: Option<(u64, Drag)>,
    message: Option<String>,
    choices: Vec<(String, String)>,
    maps: Vec<String>,
}
impl Editor {
    fn new(original: Settings) -> Self {
        Self {
            original,
            selected: None,
            tab: Tab::Basic,
            overlay: Overlay::None,
            drag: None,
            message: None,
            choices: choices(),
            maps: Vec::new(),
        }
    }
}

// ---- geometry (fractions of the screen) ----
const KEYS_PER_PAGE: usize = 18;
fn bar_rect(i: usize) -> [f32; 4] {
    [0.02 + i as f32 * 0.125, 0.025, 0.115, 0.075]
}
const BAR: [&str; 5] = ["Done", "Cancel", "Add", "Maps", "Options"];
fn panel_rect(x: f32) -> [f32; 4] {
    [x, 0.115, 0.46, 0.875]
}
fn tab_rect(x: f32, i: usize) -> [f32; 4] {
    [x + 0.01 + i as f32 * 0.15, 0.125, 0.14, 0.07]
}
fn row_rect(x: f32, i: usize) -> [f32; 4] {
    [x + 0.01, 0.215 + i as f32 * 0.085, 0.44, 0.075]
}
fn half_rect(r: [f32; 4], second: bool) -> [f32; 4] {
    let w = r[2] / 2.0 - 0.005;
    [r[0] + if second { w + 0.01 } else { 0.0 }, r[1], w, r[3]]
}
const MODAL: [f32; 4] = [0.06, 0.11, 0.88, 0.86];
fn modal_button(i: usize, count: usize) -> [f32; 4] {
    let w = (MODAL[2] - 0.04) / count as f32;
    [MODAL[0] + 0.02 + i as f32 * w, 0.88, w - 0.01, 0.07]
}
fn cat_rect(i: usize) -> [f32; 4] {
    [0.08 + i as f32 * 0.165, 0.2, 0.155, 0.07]
}
fn grid_rect(i: usize) -> [f32; 4] {
    let (col, row) = (i % 3, i / 3);
    [0.08 + col as f32 * 0.282, 0.29 + row as f32 * 0.097, 0.272, 0.085]
}
fn color_slider_rect(channel: usize) -> [f32; 4] {
    [0.2, 0.4 + channel as f32 * 0.115, 0.45, 0.07]
}
fn color_field_rect(channel: usize) -> [f32; 4] {
    [0.68, 0.4 + channel as f32 * 0.115, 0.2, 0.07]
}
const MAP_ACTIONS: [&str; 4] = ["Save", "Save as...", "Import...", "Default"];
fn map_action_rect(i: usize) -> [f32; 4] {
    [0.08 + i as f32 * 0.215, 0.2, 0.205, 0.075]
}
const MAPS_PER_PAGE: usize = 5;
fn map_row_rect(i: usize, delete: bool) -> [f32; 4] {
    let y = 0.3 + i as f32 * 0.1;
    if delete {
        [0.7, y, 0.22, 0.085]
    } else {
        [0.08, y, 0.6, 0.085]
    }
}

// ---- on-screen keyboard ----
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Act {
    Char(char),
    Back,
    Shift,
    Space,
    Ok,
    Cancel,
}
struct PadKey {
    label: String,
    act: Act,
    weight: f32,
}
fn key(c: char, shift: bool) -> PadKey {
    let c = if shift { c.to_ascii_uppercase() } else { c };
    PadKey {
        label: c.to_string(),
        act: Act::Char(c),
        weight: 1.0,
    }
}
fn special(label: &str, act: Act, weight: f32) -> PadKey {
    PadKey {
        label: label.into(),
        act,
        weight,
    }
}
fn keypad(mode: PadMode, shift: bool) -> Vec<Vec<PadKey>> {
    let chars = |s: &str, shift: bool| s.chars().map(|c| key(c, shift)).collect::<Vec<_>>();
    match mode {
        PadMode::Alpha => {
            let mut third = vec![special("Shift", Act::Shift, 1.6)];
            third.extend(chars("zxcvbnm", shift));
            third.push(special("<-", Act::Back, 1.6));
            let mut last = vec![special("Cancel", Act::Cancel, 2.0), special("Space", Act::Space, 3.0)];
            last.extend(chars("-_.", false));
            last.push(special("OK", Act::Ok, 2.0));
            vec![
                chars("1234567890", false),
                chars("qwertyuiop", shift),
                chars("asdfghjkl", shift),
                third,
                last,
            ]
        }
        PadMode::Number => vec![
            chars("789", false),
            chars("456", false),
            chars("123", false),
            vec![
                key('.', false),
                key('0', false),
                special("<-", Act::Back, 1.0),
            ],
            vec![special("Cancel", Act::Cancel, 1.0), special("OK", Act::Ok, 1.0)],
        ],
        PadMode::Hex => vec![
            chars("01234567", false),
            chars("89ABCDEF", false),
            vec![special("<-", Act::Back, 1.0)],
            vec![special("Cancel", Act::Cancel, 1.0), special("OK", Act::Ok, 1.0)],
        ],
    }
}
/// Every key's rectangle with its row and column.
fn pad_rects(rows: &[Vec<PadKey>], mode: PadMode) -> Vec<([f32; 4], usize, usize)> {
    let (left, width) = if mode == PadMode::Alpha {
        (0.04, 0.92)
    } else {
        (0.2, 0.6)
    };
    let mut out = Vec::new();
    for (r, row) in rows.iter().enumerate() {
        let total: f32 = row.iter().map(|k| k.weight).sum();
        let unit = (width - 0.008 * (row.len() as f32 - 1.0)) / total;
        let mut x = left;
        let y = 0.34 + r as f32 * 0.12;
        for (c, k) in row.iter().enumerate() {
            let w = unit * k.weight;
            out.push(([x, y, w, 0.105], r, c));
            x += w + 0.008;
        }
    }
    out
}
fn max_len(target: &Target) -> usize {
    match target {
        Target::Name => 16,
        Target::Width | Target::Height | Target::Channel(..) => 8,
        Target::Hex(_) => 8,
        Target::MapName => 32,
    }
}
fn num(v: f32) -> String {
    let s = format!("{v:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}
fn channel_shift(channel: usize) -> u32 {
    [16, 8, 0, 24][channel.min(3)]
}
fn get_channel(argb: u32, channel: usize) -> u32 {
    (argb >> channel_shift(channel)) & 0xff
}
fn set_channel(argb: u32, channel: usize, value: u32) -> u32 {
    let shift = channel_shift(channel);
    (argb & !(0xff << shift)) | (value.min(255) << shift)
}
fn parse_hex(text: &str) -> Option<u32> {
    let text = text.trim_start_matches('#');
    match text.len() {
        6 => u32::from_str_radix(text, 16).ok().map(|v| 0xFF00_0000 | v),
        8 => u32::from_str_radix(text, 16).ok(),
        _ => None,
    }
}
fn argb_to_hud(argb: u32) -> u32 {
    rgba(
        ((argb >> 16) & 0xff) as u8,
        ((argb >> 8) & 0xff) as u8,
        (argb & 0xff) as u8,
        (argb >> 24) as u8,
    )
}

impl TouchControls {
    pub fn open_editor(&mut self) {
        self.clear();
        self.buttons_hidden = false;
        self.editor = Some(Editor::new(self.settings.clone()));
    }
    pub fn cancel_editor(&mut self) {
        if let Some(e) = self.editor.take() {
            self.settings = e.original;
        }
        self.clear();
    }
    fn panel_x(&self, e: &Editor) -> f32 {
        match e.selected {
            Some(i) if self.settings.custom_buttons.get(i).is_some_and(|b| b.center[0] > 0.5) => 0.02,
            _ => 0.52,
        }
    }
    fn editor_button_at(&self, p: [f32; 2]) -> Option<usize> {
        (0..self.settings.custom_buttons.len())
            .rev()
            .find(|&i| layout::contains(self.custom_rect(i), p))
    }
    /// The editor owns every finger: editing cannot move, mine or open menus.
    /// Returns true when Done/Cancel closed the editor.
    pub fn edit_touch(&mut self, id: u64, p: [f32; 2], started: bool, ended: bool) -> bool {
        let Some(mut e) = self.editor.take() else {
            return false;
        };
        self.clicked |= started;
        let closed = self.edit_input(&mut e, id, p, started, ended);
        if !closed {
            self.editor = Some(e);
        }
        closed
    }
    fn edit_input(&mut self, e: &mut Editor, id: u64, p: [f32; 2], started: bool, ended: bool) -> bool {
        if let Some((owner, drag)) = e.drag
            && owner == id
        {
            match drag {
                Drag::Button(offset) => {
                    if let Some(i) = e.selected
                        && let Some(button) = self.settings.custom_buttons.get_mut(i)
                    {
                        button.center = [
                            (p[0] - offset[0]).clamp(0.0, 1.0),
                            (p[1] - offset[1]).clamp(0.0, 1.0),
                        ];
                    }
                }
                Drag::Slider(kind, rect) => {
                    let v = ((p[0] - rect[0]) / rect[2]).clamp(0.0, 1.0);
                    self.apply_slider(e, kind, v);
                }
            }
            if ended {
                e.drag = None;
            }
            return false;
        }
        if !started {
            return false;
        }
        if !matches!(e.overlay, Overlay::None) {
            let overlay = std::mem::replace(&mut e.overlay, Overlay::None);
            e.overlay = self.edit_overlay(e, overlay, id, p);
            return false;
        }
        e.message = None;
        for (i, label) in BAR.iter().enumerate() {
            if !layout::contains(bar_rect(i), p) {
                continue;
            }
            match *label {
                "Done" => {
                    self.settings.sanitize();
                    match self.settings.save() {
                        Ok(()) => {
                            self.clear();
                            return true;
                        }
                        Err(error) => e.message = Some(format!("Could not save: {error}")),
                    }
                }
                "Cancel" => {
                    self.settings = e.original.clone();
                    self.clear();
                    return true;
                }
                "Add" => {
                    if self.settings.custom_buttons.len() < MAX_BUTTONS {
                        let n = self.settings.custom_buttons.len();
                        self.settings.custom_buttons.push(CustomButton::new(UNKNOWN, n));
                        e.selected = Some(n);
                        e.tab = Tab::Basic;
                    } else {
                        e.message = Some(format!("At most {MAX_BUTTONS} buttons"));
                    }
                }
                "Maps" => {
                    e.maps = controlmap::list(&self.home);
                    e.overlay = Overlay::Maps {
                        page: 0,
                        confirm: None,
                    };
                }
                _ => e.overlay = Overlay::Options,
            }
            return false;
        }
        let x = self.panel_x(e);
        if let Some(i) = e.selected.filter(|&i| i < self.settings.custom_buttons.len())
            && layout::contains(panel_rect(x), p)
        {
            self.edit_panel(e, i, x, id, p);
            return false;
        }
        match self.editor_button_at(p) {
            Some(i) => {
                e.selected = Some(i);
                let c = self.settings.custom_buttons[i].center;
                e.drag = Some((id, Drag::Button([p[0] - c[0], p[1] - c[1]])));
            }
            None => e.selected = None,
        }
        false
    }
    fn apply_slider(&mut self, e: &Editor, kind: Slider, v: f32) {
        match kind {
            Slider::ButtonScale => self.settings.button_scale = (0.5 + v * 1.5).clamp(0.5, 2.0),
            _ => {
                let Some(button) = e.selected.and_then(|i| self.settings.custom_buttons.get_mut(i)) else {
                    return;
                };
                match kind {
                    Slider::Opacity => button.opacity = v,
                    Slider::StrokeWidth => button.stroke_width = (v * 100.0).round(),
                    Slider::CornerRadius => button.corner_radius = (v * 100.0).round(),
                    Slider::Channel(which, c) => {
                        let slot = match which {
                            Which::Background => &mut button.background,
                            Which::Stroke => &mut button.stroke_color,
                        };
                        *slot = set_channel(*slot, c, (v * 255.0).round() as u32);
                    }
                    Slider::ButtonScale => {}
                }
            }
        }
    }
    fn color_of(&self, e: &Editor, which: Which) -> u32 {
        e.selected
            .and_then(|i| self.settings.custom_buttons.get(i))
            .map_or(0, |b| match which {
                Which::Background => b.background,
                Which::Stroke => b.stroke_color,
            })
    }
    fn set_color(&mut self, e: &Editor, which: Which, argb: u32) {
        if let Some(b) = e.selected.and_then(|i| self.settings.custom_buttons.get_mut(i)) {
            match which {
                Which::Background => b.background = argb,
                Which::Stroke => b.stroke_color = argb,
            }
        }
    }
    fn open_text(&self, e: &mut Editor, target: Target, title: &str, initial: String, mode: PadMode) {
        e.overlay = Overlay::Text(TextInput {
            target,
            mode,
            buffer: initial,
            shift: false,
            title: title.into(),
        });
    }
    fn edit_panel(&mut self, e: &mut Editor, i: usize, x: f32, id: u64, p: [f32; 2]) {
        for (t, (tab, _)) in TABS.iter().enumerate() {
            if layout::contains(tab_rect(x, t), p) {
                e.tab = *tab;
                return;
            }
        }
        for (r, row) in rows(e.tab).into_iter().enumerate() {
            let rect = row_rect(x, r);
            if !layout::contains(rect, p) {
                continue;
            }
            let button = self.settings.custom_buttons[i].clone();
            match row {
                Row::Name => self.open_text(e, Target::Name, "Button name", button.name.clone(), PadMode::Alpha),
                Row::Width => self.open_text(e, Target::Width, "Width (dp)", num(button.width), PadMode::Number),
                Row::Height => self.open_text(e, Target::Height, "Height (dp)", num(button.height), PadMode::Number),
                Row::Key(slot) => {
                    let current = button.keys.get(slot).map_or(UNKNOWN, String::as_str);
                    let cat = category(current);
                    e.overlay = Overlay::Keys { slot, cat, page: 0 };
                }
                Row::CloneDelete => {
                    if layout::contains(half_rect(rect, false), p) {
                        if self.settings.custom_buttons.len() < MAX_BUTTONS {
                            let mut copy = button;
                            copy.center = [(copy.center[0] + 0.03).min(1.0), (copy.center[1] + 0.05).min(1.0)];
                            self.settings.custom_buttons.push(copy);
                            e.selected = Some(self.settings.custom_buttons.len() - 1);
                        } else {
                            e.message = Some(format!("At most {MAX_BUTTONS} buttons"));
                        }
                    } else {
                        self.settings.custom_buttons.remove(i);
                        e.selected = None;
                    }
                }
                Row::Toggle => self.settings.custom_buttons[i].toggle ^= true,
                Row::Swipe => self.settings.custom_buttons[i].swipeable ^= true,
                Row::PassThrough => self.settings.custom_buttons[i].pass_through ^= true,
                Row::InGame => self.settings.custom_buttons[i].show_in_game ^= true,
                Row::InMenu => self.settings.custom_buttons[i].show_in_menu ^= true,
                Row::Opacity => {
                    e.drag = Some((id, Drag::Slider(Slider::Opacity, rect)));
                    self.apply_slider(e, Slider::Opacity, ((p[0] - rect[0]) / rect[2]).clamp(0.0, 1.0));
                }
                Row::Color(which) => e.overlay = Overlay::Color(which),
                Row::StrokeWidth => {
                    e.drag = Some((id, Drag::Slider(Slider::StrokeWidth, rect)));
                    self.apply_slider(e, Slider::StrokeWidth, ((p[0] - rect[0]) / rect[2]).clamp(0.0, 1.0));
                }
                Row::CornerRadius => {
                    e.drag = Some((id, Drag::Slider(Slider::CornerRadius, rect)));
                    self.apply_slider(e, Slider::CornerRadius, ((p[0] - rect[0]) / rect[2]).clamp(0.0, 1.0));
                }
            }
            return;
        }
    }
    fn edit_overlay(&mut self, e: &mut Editor, overlay: Overlay, id: u64, p: [f32; 2]) -> Overlay {
        match overlay {
            Overlay::None => Overlay::None,
            Overlay::Keys { slot, mut cat, mut page } => {
                for (c, (kind, _)) in CATS.iter().enumerate() {
                    if layout::contains(cat_rect(c), p) {
                        cat = *kind;
                        page = 0;
                        return Overlay::Keys { slot, cat, page };
                    }
                }
                let list: Vec<&(String, String)> = e.choices.iter().filter(|(t, _)| category(t) == cat).collect();
                let pages = list.len().div_ceil(KEYS_PER_PAGE).max(1);
                if layout::contains(modal_button(0, 3), p) {
                    return Overlay::Keys { slot, cat, page: (page + pages - 1) % pages };
                }
                if layout::contains(modal_button(1, 3), p) {
                    return Overlay::Keys { slot, cat, page: (page + 1) % pages };
                }
                if layout::contains(modal_button(2, 3), p) {
                    return Overlay::None;
                }
                for n in 0..KEYS_PER_PAGE {
                    if layout::contains(grid_rect(n), p)
                        && let Some((token, _)) = list.get(page * KEYS_PER_PAGE + n)
                        && let Some(button) = e.selected.and_then(|b| self.settings.custom_buttons.get_mut(b))
                    {
                        while button.keys.len() <= slot.min(MAX_KEYS - 1) {
                            button.keys.push(UNKNOWN.into());
                        }
                        button.keys[slot] = token.clone();
                        return Overlay::None;
                    }
                }
                Overlay::Keys { slot, cat, page }
            }
            Overlay::Text(mut input) => {
                let rows = keypad(input.mode, input.shift);
                for (rect, r, c) in pad_rects(&rows, input.mode) {
                    if !layout::contains(rect, p) {
                        continue;
                    }
                    match rows[r][c].act {
                        Act::Char(ch) => {
                            if input.buffer.chars().count() < max_len(&input.target)
                                && (input.mode != PadMode::Number || ch != '.' || !input.buffer.contains('.'))
                            {
                                input.buffer.push(ch);
                            }
                            input.shift = false;
                        }
                        Act::Space => {
                            if input.buffer.chars().count() < max_len(&input.target) {
                                input.buffer.push(' ');
                            }
                        }
                        Act::Back => {
                            input.buffer.pop();
                        }
                        Act::Shift => input.shift = !input.shift,
                        Act::Cancel => return after_text(&input.target),
                        Act::Ok => {
                            self.finish_text(e, &input);
                            return after_text(&input.target);
                        }
                    }
                    break;
                }
                Overlay::Text(input)
            }
            Overlay::Color(which) => {
                if layout::contains(modal_button(1, 3), p) {
                    return Overlay::None;
                }
                let argb = self.color_of(e, which);
                if layout::contains([0.38, 0.2, 0.5, 0.08], p) {
                    self.open_text(e, Target::Hex(which), which.title(), format!("{argb:08X}"), PadMode::Hex);
                    return std::mem::replace(&mut e.overlay, Overlay::None);
                }
                for c in 0..4 {
                    let slider = color_slider_rect(c);
                    if layout::contains(slider, p) {
                        e.drag = Some((id, Drag::Slider(Slider::Channel(which, c), slider)));
                        self.apply_slider(e, Slider::Channel(which, c), ((p[0] - slider[0]) / slider[2]).clamp(0.0, 1.0));
                    } else if layout::contains(color_field_rect(c), p) {
                        self.open_text(e, Target::Channel(which, c), "0 - 255", get_channel(argb, c).to_string(), PadMode::Number);
                        return std::mem::replace(&mut e.overlay, Overlay::None);
                    }
                }
                Overlay::Color(which)
            }
            Overlay::Maps { page, confirm } => self.edit_maps(e, page, confirm, p),
            Overlay::Options => {
                if layout::contains(modal_button(1, 3), p) {
                    return Overlay::None;
                }
                if layout::contains([0.1, 0.25, 0.8, 0.09], p) {
                    self.settings.crosshair_gestures ^= true;
                }
                let slider = [0.1, 0.5, 0.8, 0.09];
                if layout::contains(slider, p) {
                    e.drag = Some((id, Drag::Slider(Slider::ButtonScale, slider)));
                    self.apply_slider(e, Slider::ButtonScale, ((p[0] - slider[0]) / slider[2]).clamp(0.0, 1.0));
                }
                Overlay::Options
            }
        }
    }
    fn finish_text(&mut self, e: &mut Editor, input: &TextInput) {
        let text = input.buffer.trim();
        match &input.target {
            Target::Name => {
                if let Some(b) = e.selected.and_then(|i| self.settings.custom_buttons.get_mut(i)) {
                    b.name = text.to_owned();
                }
            }
            Target::Width | Target::Height => match text.parse::<f32>() {
                Ok(v) if v.is_finite() => {
                    if let Some(b) = e.selected.and_then(|i| self.settings.custom_buttons.get_mut(i)) {
                        let v = v.clamp(8.0, 600.0);
                        if input.target == Target::Width {
                            b.width = v;
                        } else {
                            b.height = v;
                        }
                    }
                }
                _ => e.message = Some("Not a number".into()),
            },
            Target::Hex(which) => match parse_hex(text) {
                Some(argb) => self.set_color(e, *which, argb),
                None => e.message = Some("Use RRGGBB or AARRGGBB".into()),
            },
            Target::Channel(which, c) => match text.parse::<u32>() {
                Ok(v) if v <= 255 => {
                    let argb = self.color_of(e, *which);
                    self.set_color(e, *which, set_channel(argb, *c, v));
                }
                _ => e.message = Some("Use 0 to 255".into()),
            },
            Target::MapName => match controlmap::save(&self.home, text, &self.settings.custom_buttons) {
                Ok(name) => {
                    e.message = Some(format!("Saved map {name}"));
                    self.settings.controlmap = name;
                    e.maps = controlmap::list(&self.home);
                }
                Err(error) => e.message = Some(error),
            },
        }
    }
    fn edit_maps(&mut self, e: &mut Editor, page: usize, confirm: Option<usize>, p: [f32; 2]) -> Overlay {
        let pages = e.maps.len().div_ceil(MAPS_PER_PAGE).max(1);
        if layout::contains(modal_button(2, 3), p) {
            return Overlay::None;
        }
        if layout::contains(modal_button(0, 3), p) {
            return Overlay::Maps { page: (page + pages - 1) % pages, confirm: None };
        }
        if layout::contains(modal_button(1, 3), p) {
            return Overlay::Maps { page: (page + 1) % pages, confirm: None };
        }
        for (i, label) in MAP_ACTIONS.iter().enumerate() {
            if !layout::contains(map_action_rect(i), p) {
                continue;
            }
            match *label {
                "Save" => {
                    let name = self.settings.controlmap.clone();
                    match controlmap::save(&self.home, &name, &self.settings.custom_buttons) {
                        Ok(name) => e.message = Some(format!("Saved map {name}")),
                        Err(error) => e.message = Some(error),
                    }
                    e.maps = controlmap::list(&self.home);
                }
                "Save as..." => {
                    self.open_text(e, Target::MapName, "Save map as", String::new(), PadMode::Alpha);
                    return std::mem::replace(&mut e.overlay, Overlay::None);
                }
                "Import..." => {
                    self.import_requested = true;
                    e.message = Some("Choose a Pojav or Horizon layout file, then reopen Maps".into());
                    return Overlay::None;
                }
                _ => {
                    self.settings.custom_buttons = controlmap::default_buttons();
                    self.settings.controlmap = controlmap::DEFAULT_NAME.into();
                    e.selected = None;
                    e.message = Some("Default layout restored".into());
                }
            }
            return Overlay::Maps { page, confirm: None };
        }
        for n in 0..MAPS_PER_PAGE {
            let index = page * MAPS_PER_PAGE + n;
            let Some(name) = e.maps.get(index).cloned() else {
                break;
            };
            if layout::contains(map_row_rect(n, true), p) {
                if confirm == Some(index) {
                    match controlmap::delete(&self.home, &name) {
                        Ok(()) => e.message = Some(format!("Deleted {name}")),
                        Err(error) => e.message = Some(error),
                    }
                    e.maps = controlmap::list(&self.home);
                    return Overlay::Maps { page: 0, confirm: None };
                }
                return Overlay::Maps { page, confirm: Some(index) };
            }
            if layout::contains(map_row_rect(n, false), p) {
                let density = if self.density > 0.0 { self.density } else { 2.75 };
                match controlmap::load(&self.home, &name, self.surface_px(), density) {
                    Ok(buttons) => {
                        self.settings.custom_buttons = buttons;
                        self.settings.controlmap = name.clone();
                        self.settings.sanitize();
                        e.selected = None;
                        e.message = Some(format!("Loaded {name}"));
                        return Overlay::None;
                    }
                    Err(error) => e.message = Some(format!("{name}: {error}")),
                }
                return Overlay::Maps { page, confirm: None };
            }
        }
        Overlay::Maps { page, confirm }
    }
    pub fn capture_custom_key(&mut self, name: &str) -> bool {
        if crate::keybinds::key_code(name).is_none() {
            return false;
        }
        let Some(e) = &mut self.editor else {
            return false;
        };
        let Overlay::Keys { slot, .. } = e.overlay else {
            return false;
        };
        let Some(button) = e.selected.and_then(|i| self.settings.custom_buttons.get_mut(i)) else {
            return false;
        };
        while button.keys.len() <= slot.min(MAX_KEYS - 1) {
            button.keys.push(UNKNOWN.into());
        }
        button.keys[slot] = name.into();
        e.overlay = Overlay::None;
        true
    }

    // ---- drawing ----
    pub(super) fn draw_editor(&self, b: &mut HudBatch, a: &HudAtlas) {
        let Some(e) = &self.editor else {
            return;
        };
        let (w, h) = (b.width, b.height);
        for (i, label) in BAR.iter().enumerate() {
            plate(b, a, bar_rect(i), label, false);
        }
        if let Some(message) = &e.message {
            b.text(a, message, 0.02 * w, 0.11 * h, rgba(255, 220, 120, 255), true);
        } else if e.selected.is_none() && matches!(e.overlay, Overlay::None) {
            b.text(
                a,
                "Tap a button to edit it, drag to move",
                0.02 * w,
                0.11 * h,
                rgba(255, 255, 255, 220),
                true,
            );
        }
        if let Some(i) = e.selected.filter(|&i| i < self.settings.custom_buttons.len()) {
            let [rx, ry, rw, rh] = self.custom_rect(i);
            let (x, y, bw, bh) = (rx * w - 1.0, ry * h - 1.0, rw * w + 2.0, rh * h + 2.0);
            let white = rgba(255, 255, 255, 255);
            for (p, q) in [
                ([x, y], [x + bw, y]),
                ([x, y + bh], [x + bw, y + bh]),
                ([x, y], [x, y + bh]),
                ([x + bw, y], [x + bw, y + bh]),
            ] {
                line(b, a, p, q, white);
            }
            if matches!(e.overlay, Overlay::None) {
                self.draw_panel(b, a, e, i);
            }
        }
        match &e.overlay {
            Overlay::None => {}
            Overlay::Keys { slot, cat, page } => self.draw_keys(b, a, e, *slot, *cat, *page),
            Overlay::Text(input) => draw_text(b, a, input),
            Overlay::Color(which) => self.draw_color(b, a, e, *which),
            Overlay::Maps { page, confirm } => self.draw_maps(b, a, e, *page, *confirm),
            Overlay::Options => self.draw_options(b, a),
        }
    }
    fn draw_panel(&self, b: &mut HudBatch, a: &HudAtlas, e: &Editor, i: usize) {
        let (w, h) = (b.width, b.height);
        let x = self.panel_x(e);
        let panel = panel_rect(x);
        b.fill(a, panel[0] * w, panel[1] * h, panel[2] * w, panel[3] * h, rgba(0, 0, 0, 190));
        for (t, (tab, label)) in TABS.iter().enumerate() {
            plate(b, a, tab_rect(x, t), label, e.tab == *tab);
        }
        let button = &self.settings.custom_buttons[i];
        let onoff = |v: bool| if v { "ON" } else { "OFF" };
        for (r, row) in rows(e.tab).into_iter().enumerate() {
            let rect = row_rect(x, r);
            match row {
                Row::Name => plate(b, a, rect, &format!("Name: {}", if button.name.is_empty() { "-" } else { button.name.as_str() }), false),
                Row::Width => plate(b, a, rect, &format!("Width: {} dp", num(button.width)), false),
                Row::Height => plate(b, a, rect, &format!("Height: {} dp", num(button.height)), false),
                Row::Key(slot) => {
                    let token = button.keys.get(slot).map_or(UNKNOWN, String::as_str);
                    plate(b, a, rect, &format!("Key {}: {}", slot + 1, CustomButton::token_label(token)), false);
                }
                Row::CloneDelete => {
                    plate(b, a, half_rect(rect, false), "Clone", false);
                    plate(b, a, half_rect(rect, true), "Delete", false);
                }
                Row::Toggle => plate(b, a, rect, &format!("Toggle: {}", onoff(button.toggle)), button.toggle),
                Row::Swipe => plate(b, a, rect, &format!("Swipeable: {}", onoff(button.swipeable)), button.swipeable),
                Row::PassThrough => plate(b, a, rect, &format!("Mouse pass-through: {}", onoff(button.pass_through)), button.pass_through),
                Row::InGame => plate(b, a, rect, &format!("In game: {}", onoff(button.show_in_game)), button.show_in_game),
                Row::InMenu => plate(b, a, rect, &format!("In menu: {}", onoff(button.show_in_menu)), button.show_in_menu),
                Row::Opacity => slider(b, a, rect, &format!("Opacity: {}%", (button.opacity * 100.0).round()), button.opacity),
                Row::Color(which) => {
                    let argb = match which {
                        Which::Background => button.background,
                        Which::Stroke => button.stroke_color,
                    };
                    plate(b, a, rect, which.title(), false);
                    let swatch = [rect[0] + rect[2] - 0.075, rect[1] + 0.012, 0.06, rect[3] - 0.024];
                    b.fill(a, swatch[0] * w - 1.0, swatch[1] * h - 1.0, swatch[2] * w + 2.0, swatch[3] * h + 2.0, rgba(0, 0, 0, 255));
                    b.fill(a, swatch[0] * w, swatch[1] * h, swatch[2] * w, swatch[3] * h, rgba(255, 255, 255, 255));
                    b.fill(a, swatch[0] * w, swatch[1] * h, swatch[2] * w, swatch[3] * h, argb_to_hud(argb));
                }
                Row::StrokeWidth => slider(b, a, rect, &format!("Stroke width: {}%", button.stroke_width.round()), button.stroke_width / 100.0),
                Row::CornerRadius => slider(b, a, rect, &format!("Corner radius: {}%", button.corner_radius.round()), button.corner_radius / 100.0),
            }
        }
    }
    fn draw_modal(&self, b: &mut HudBatch, a: &HudAtlas, title: &str) {
        let (w, h) = (b.width, b.height);
        b.fill(a, 0.0, 0.0, w, h, rgba(0, 0, 0, 120));
        b.fill(a, MODAL[0] * w, MODAL[1] * h, MODAL[2] * w, MODAL[3] * h, rgba(20, 20, 20, 235));
        b.text(a, title, (MODAL[0] + 0.02) * w, (MODAL[1] + 0.025) * h, rgba(255, 255, 255, 255), true);
    }
    fn draw_keys(&self, b: &mut HudBatch, a: &HudAtlas, e: &Editor, slot: usize, cat: Cat, page: usize) {
        self.draw_modal(b, a, &format!("Key {} - tap a key here, or press one on a keyboard", slot + 1));
        for (c, (kind, label)) in CATS.iter().enumerate() {
            plate(b, a, cat_rect(c), label, *kind == cat);
        }
        let list: Vec<&(String, String)> = e.choices.iter().filter(|(t, _)| category(t) == cat).collect();
        let pages = list.len().div_ceil(KEYS_PER_PAGE).max(1);
        let current = e
            .selected
            .and_then(|i| self.settings.custom_buttons.get(i))
            .and_then(|btn| btn.keys.get(slot))
            .map_or(UNKNOWN, String::as_str);
        for n in 0..KEYS_PER_PAGE {
            if let Some((token, label)) = list.get(page * KEYS_PER_PAGE + n) {
                plate(b, a, grid_rect(n), label, token == current);
            }
        }
        plate(b, a, modal_button(0, 3), "Prev", false);
        plate(b, a, modal_button(1, 3), &format!("Next ({}/{})", page.min(pages - 1) + 1, pages), false);
        plate(b, a, modal_button(2, 3), "Close", false);
    }
    fn draw_color(&self, b: &mut HudBatch, a: &HudAtlas, e: &Editor, which: Which) {
        let (w, h) = (b.width, b.height);
        self.draw_modal(b, a, which.title());
        let argb = self.color_of(e, which);
        let swatch = [0.12, 0.2, 0.22, 0.12];
        b.fill(a, swatch[0] * w - 1.0, swatch[1] * h - 1.0, swatch[2] * w + 2.0, swatch[3] * h + 2.0, rgba(255, 255, 255, 255));
        b.fill(a, swatch[0] * w, swatch[1] * h, swatch[2] * w, swatch[3] * h, rgba(60, 60, 60, 255));
        b.fill(a, swatch[0] * w, swatch[1] * h, swatch[2] * w, swatch[3] * h, argb_to_hud(argb));
        plate(b, a, [0.38, 0.2, 0.5, 0.08], &format!("Hex: #{argb:08X}"), false);
        b.text(a, "ARGB. Tap the hex or a number to type it.", 0.38 * w, 0.3 * h, rgba(200, 200, 200, 255), true);
        for (c, name) in ["R", "G", "B", "A"].iter().enumerate() {
            let rect = color_slider_rect(c);
            b.text(a, name, 0.14 * w, (rect[1] + 0.025) * h, rgba(255, 255, 255, 255), true);
            slider(b, a, rect, "", get_channel(argb, c) as f32 / 255.0);
            plate(b, a, color_field_rect(c), &get_channel(argb, c).to_string(), false);
        }
        plate(b, a, modal_button(1, 3), "Close", false);
    }
    fn draw_maps(&self, b: &mut HudBatch, a: &HudAtlas, e: &Editor, page: usize, confirm: Option<usize>) {
        self.draw_modal(b, a, &format!("Control maps - current: {}", self.settings.controlmap));
        for (i, label) in MAP_ACTIONS.iter().enumerate() {
            plate(b, a, map_action_rect(i), label, false);
        }
        let pages = e.maps.len().div_ceil(MAPS_PER_PAGE).max(1);
        if e.maps.is_empty() {
            b.text(a, "No saved maps yet. Use Save as... or Import...", 0.08 * b.width, 0.32 * b.height, rgba(200, 200, 200, 255), true);
        }
        for n in 0..MAPS_PER_PAGE {
            let index = page * MAPS_PER_PAGE + n;
            let Some(name) = e.maps.get(index) else {
                break;
            };
            plate(b, a, map_row_rect(n, false), name, *name == self.settings.controlmap);
            plate(b, a, map_row_rect(n, true), if confirm == Some(index) { "Sure?" } else { "Delete" }, confirm == Some(index));
        }
        plate(b, a, modal_button(0, 3), "Prev", false);
        plate(b, a, modal_button(1, 3), &format!("Next ({}/{})", page.min(pages - 1) + 1, pages), false);
        plate(b, a, modal_button(2, 3), "Close", false);
    }
    fn draw_options(&self, b: &mut HudBatch, a: &HudAtlas) {
        self.draw_modal(b, a, "Options");
        let off = self.settings.crosshair_gestures;
        plate(
            b,
            a,
            [0.1, 0.25, 0.8, 0.09],
            &format!("Disable gestures: {}", if off { "OFF" } else { "ON" }),
            !off,
        );
        b.text(
            a,
            "Gestures: tap to place, hold to mine, tap the hotbar. Turning the view always works.",
            0.1 * b.width,
            0.36 * b.height,
            rgba(200, 200, 200, 255),
            true,
        );
        slider(
            b,
            a,
            [0.1, 0.5, 0.8, 0.09],
            &format!("Button size: {}%", (self.settings.button_scale * 100.0).round()),
            (self.settings.button_scale - 0.5) / 1.5,
        );
        plate(b, a, modal_button(1, 3), "Close", false);
    }
}
fn after_text(target: &Target) -> Overlay {
    match target {
        Target::Hex(which) | Target::Channel(which, _) => Overlay::Color(*which),
        _ => Overlay::None,
    }
}
fn draw_text(b: &mut HudBatch, a: &HudAtlas, input: &TextInput) {
    let (w, h) = (b.width, b.height);
    b.fill(a, 0.0, 0.0, w, h, rgba(0, 0, 0, 150));
    b.text(a, &input.title, 0.2 * w, 0.12 * h, rgba(255, 255, 255, 255), true);
    b.fill(a, 0.2 * w, 0.2 * h, 0.6 * w, 0.1 * h, rgba(255, 255, 255, 255));
    b.fill(a, 0.2 * w + 1.0, 0.2 * h + 1.0, 0.6 * w - 2.0, 0.1 * h - 2.0, rgba(15, 15, 15, 255));
    b.text(a, &format!("{}_", input.buffer), 0.21 * w, 0.2 * h + (0.1 * h - 8.0) / 2.0, rgba(255, 255, 255, 255), false);
    let rows = keypad(input.mode, input.shift);
    for (rect, r, c) in pad_rects(&rows, input.mode) {
        let key = &rows[r][c];
        plate(b, a, rect, &key.label, key.act == Act::Shift && input.shift);
    }
}
fn slider(b: &mut HudBatch, a: &HudAtlas, [x, y, w, h]: [f32; 4], label: &str, value: f32) {
    let (bw, bh) = (b.width, b.height);
    let (px, py, pw, ph) = (x * bw, y * bh, w * bw, h * bh);
    let value = if value.is_finite() { value.clamp(0.0, 1.0) } else { 0.0 };
    b.fill(a, px, py, pw, ph, rgba(255, 255, 255, 255));
    b.fill(a, px + 1.0, py + 1.0, pw - 2.0, ph - 2.0, rgba(40, 40, 40, 255));
    b.fill(a, px + 1.0, py + 1.0, (pw - 2.0) * value, ph - 2.0, rgba(80, 170, 70, 255));
    let knob = px + (pw - 4.0) * value;
    b.fill(a, knob, py - 1.0, 4.0, ph + 2.0, rgba(255, 255, 255, 255));
    if !label.is_empty() {
        let mut text = label.to_owned();
        while HudBatch::text_width(a, &text) > pw - 6.0 && !text.is_empty() {
            text.pop();
        }
        b.text(a, &text, px + 3.0, py + (ph - 8.0) / 2.0, rgba(255, 255, 255, 255), true);
    }
}
fn plate(b: &mut HudBatch, a: &HudAtlas, [x, y, w, h]: [f32; 4], label: &str, on: bool) {
    let (bw, bh) = (b.width, b.height);
    nine(
        b,
        a,
        "ui/button_borderless_light",
        1.0,
        [x * bw, y * bh, w * bw, h * bh],
        if on {
            rgba(100, 210, 90, 255)
        } else {
            rgba(255, 255, 255, 255)
        },
    );
    let mut display = label.to_owned();
    while HudBatch::text_width(a, &display) > w * bw - 6.0 && !display.is_empty() {
        display.pop();
    }
    b.text(
        a,
        &display,
        x * bw + 3.0,
        y * bh + (h * bh - 8.0) / 2.0,
        rgba(35, 35, 35, 255),
        false,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    fn editing() -> TouchControls {
        let mut t = TouchControls::default();
        t.viewport(1920, 1080);
        t.density = 2.0;
        t.settings.custom_buttons = vec![CustomButton::new("key.keyboard.z", 0)];
        t.open_editor();
        t
    }
    fn tap(t: &mut TouchControls, p: [f32; 2]) -> bool {
        let closed = t.edit_touch(9, p, true, false);
        t.edit_touch(9, p, false, true);
        closed
    }
    fn centre(r: [f32; 4]) -> [f32; 2] {
        [r[0] + r[2] / 2.0, r[1] + r[3] / 2.0]
    }
    fn select_first(t: &mut TouchControls) {
        let c = t.settings.custom_buttons[0].center;
        tap(t, c);
        assert_eq!(t.editor.as_ref().unwrap().selected, Some(0));
    }
    #[test]
    fn tapping_a_button_selects_it_and_dragging_moves_it() {
        let mut t = editing();
        let c = t.settings.custom_buttons[0].center;
        t.edit_touch(1, c, true, false);
        t.edit_touch(1, [c[0] + 0.1, c[1] + 0.2], false, false);
        t.edit_touch(1, [c[0] + 0.1, c[1] + 0.2], false, true);
        let moved = t.settings.custom_buttons[0].center;
        assert!((moved[0] - c[0] - 0.1).abs() < 1e-5 && (moved[1] - c[1] - 0.2).abs() < 1e-5);
        assert_eq!(t.editor.as_ref().unwrap().selected, Some(0));
    }
    #[test]
    fn add_clone_and_delete_manage_the_list() {
        let mut t = editing();
        tap(&mut t, centre(bar_rect(2)));
        assert_eq!(t.settings.custom_buttons.len(), 2);
        assert_eq!(t.editor.as_ref().unwrap().selected, Some(1));
        let x = t.panel_x(t.editor.as_ref().unwrap());
        let clone = half_rect(row_rect(x, 7), false);
        tap(&mut t, centre(clone));
        assert_eq!(t.settings.custom_buttons.len(), 3);
        let x = t.panel_x(t.editor.as_ref().unwrap());
        let delete = half_rect(row_rect(x, 7), true);
        tap(&mut t, centre(delete));
        assert_eq!(t.settings.custom_buttons.len(), 2);
        assert_eq!(t.editor.as_ref().unwrap().selected, None);
    }
    #[test]
    fn behaviour_rows_toggle_the_flags() {
        let mut t = editing();
        select_first(&mut t);
        let x = t.panel_x(t.editor.as_ref().unwrap());
        tap(&mut t, centre(tab_rect(x, 1)));
        for (row, check) in [
            (0usize, 0usize),
            (1, 1),
            (2, 2),
            (3, 3),
            (4, 4),
        ] {
            tap(&mut t, centre(row_rect(x, row)));
            let b = &t.settings.custom_buttons[0];
            let flags = [b.toggle, b.swipeable, b.pass_through, !b.show_in_game, !b.show_in_menu];
            assert!(flags[check], "row {row}");
        }
    }
    #[test]
    fn typing_a_size_applies_and_rejects_nonsense() {
        let mut t = editing();
        select_first(&mut t);
        let x = t.panel_x(t.editor.as_ref().unwrap());
        tap(&mut t, centre(row_rect(x, 1)));
        let keys = keypad(PadMode::Number, false);
        let rects = pad_rects(&keys, PadMode::Number);
        let press = |t: &mut TouchControls, label: &str| {
            let (rect, _, _) = rects
                .iter()
                .find(|(_, r, c)| keys[*r][*c].label == label)
                .copied()
                .unwrap();
            tap(t, centre(rect));
        };
        for _ in 0..4 {
            press(&mut t, "<-");
        }
        for label in ["2", "2", "5", ".", "5"] {
            press(&mut t, label);
        }
        press(&mut t, "OK");
        assert_eq!(t.settings.custom_buttons[0].width, 225.5);
        assert!(matches!(t.editor.as_ref().unwrap().overlay, Overlay::None));
    }
    #[test]
    fn colours_parse_from_hex_and_channels() {
        assert_eq!(parse_hex("4D000000"), Some(0x4D00_0000));
        assert_eq!(parse_hex("#FF8800"), Some(0xFFFF_8800));
        assert_eq!(parse_hex("12"), None);
        assert_eq!(set_channel(0xFF00_0000, 0, 0x80), 0xFF80_0000);
        assert_eq!(get_channel(0xFF80_4020, 3), 0xFF);
        assert_eq!(get_channel(0xFF80_4020, 1), 0x40);
        assert_eq!(set_channel(0x1122_3344, 3, 0x99), 0x9922_3344);
    }
    #[test]
    fn key_slots_take_picked_and_captured_keys() {
        let mut t = editing();
        select_first(&mut t);
        let x = t.panel_x(t.editor.as_ref().unwrap());
        tap(&mut t, centre(row_rect(x, 4)));
        assert!(matches!(t.editor.as_ref().unwrap().overlay, Overlay::Keys { slot: 1, .. }));
        assert!(t.capture_custom_key("key.keyboard.x"));
        assert_eq!(t.settings.custom_buttons[0].keys, ["key.keyboard.z", "key.keyboard.x"]);
        assert!(!t.capture_custom_key("key.keyboard.y"));
    }
    #[test]
    fn cancel_closes_and_restores_the_original() {
        let mut t = editing();
        let c = t.settings.custom_buttons[0].center;
        t.edit_touch(1, c, true, false);
        t.edit_touch(1, [0.9, 0.9], false, false);
        t.edit_touch(1, [0.9, 0.9], false, true);
        assert!(tap(&mut t, centre(bar_rect(1))));
        assert!(t.editor.is_none());
        assert_eq!(t.settings.custom_buttons[0].center, c);
    }
    #[test]
    fn the_panel_opens_on_the_side_away_from_the_button() {
        let mut t = editing();
        t.settings.custom_buttons[0].center = [0.8, 0.5];
        select_first(&mut t);
        assert_eq!(t.panel_x(t.editor.as_ref().unwrap()), 0.02);
        t.settings.custom_buttons[0].center = [0.2, 0.5];
        assert_eq!(t.panel_x(t.editor.as_ref().unwrap()), 0.52);
    }
    #[test]
    fn categories_cover_every_choice() {
        let all = choices();
        assert!(all.len() > 50);
        for (token, _) in &all {
            let cat = category(token);
            assert!(CATS.iter().any(|(c, _)| *c == cat));
        }
        assert_eq!(category("key.keyboard.w"), Cat::Letters);
        assert_eq!(category("key.keyboard.f5"), Cat::Numbers);
        assert_eq!(category("key.keyboard.space"), Cat::Keys);
        assert_eq!(category("action:key.jump"), Cat::Actions);
        assert_eq!(category(HIDE_BUTTONS), Cat::Special);
    }
}
