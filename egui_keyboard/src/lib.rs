#![doc = include_str!("../README.md")]

// const SHIFT_CHARACTERS: [[char; 2]; 7] = [['⬆', '⇧'], ['⬆', '⬆'], ['⇧', '⇧'], ['▲', '△'], ['▲', '▲'], ['△', '△'], ['^', '^']];
// const BACKSPACE_CHARACTERS: [char; 4] = ['⌫', '◁', '◀', '<'];

mod clipboard;
pub mod layouts;

use crate::layouts::KeyboardLayout;
use egui::{
    vec2, Align2, Button, Context, Event, Frame, Id, Modifiers, Order, Rect, Ui, Vec2, WidgetText,
    Window,
};
use std::collections::VecDeque;

enum Key {
    Text(&'static str),
    Backspace,
    Upper,
    Space,
    Special
}

impl Key {
    pub(crate) fn width_relative(&self) -> f32 {
        match self {
            Self::Text(_) => 1.0,
            Self::Backspace => 1.5,
            Self::Upper => 1.5,
            Self::Space => 0.0,
            Self::Special => 1.5
        }
    }
}

const SPACE_BETWEEN_KEYS: f32 = 1.0 / 6.0;

/// Main struct for the virtual keyboard. It stores the state of the keyboard and handles the
/// rendering. Needs to be stored between frames.
#[derive(Default)]
pub struct Keyboard {
    input_widget: Option<Id>,
    events: VecDeque<Event>,
    upper: bool,
    special: bool,
    keyboard_layout: KeyboardLayout,

    shift_characters: [char; 2],
    backspace_character: char,

    /// How much keyboard is needed. It's a number so we can implement this as some sort of
    /// hysteresis to avoid flickering.
    needed: u32,

    /// Last rect where the keyboard was rendered.
    last_rect: Option<Rect>,
}

impl Keyboard {
    pub fn new(shift_characters: [char; 2], backspace_character: char) -> Self {
        Self {
            shift_characters,
            backspace_character,
            ..Default::default()
        }
    }
}

fn heading_button(text: &str, button_size: Option<Vec2>) -> Button<'static> {
    button(WidgetText::from(text).heading(), button_size)
}

fn button(text: impl Into<WidgetText>, button_size: Option<Vec2>) -> Button<'static> {
    let mut button = Button::new(text).frame(true);
    if button_size.is_none() {
        button = button.min_size(Vec2::new(10.0, 50.0))
    }
    button
}

impl Keyboard {
    /// Inject text events into Egui context. This function needs to be called before any widget is
    /// created, otherwise the key presses will be ignored.
    pub fn pump_events(&mut self, ctx: &Context) {
        ctx.input_mut(|input| input.events.extend(std::mem::take(&mut self.events)));
    }

    pub fn layout(mut self, layout: KeyboardLayout) -> Self {
        self.keyboard_layout = layout;
        self
    }

    /// Area which is free from the keyboard. This is useful when you want to constrain a window to
    /// the area which is not covered by the keyboard.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # egui::__run_test_ctx(|ctx| {
    /// # let keyboard = egui_keyboard::Keyboard::default();
    /// egui::Window::new("Hello")
    ///   .constrain_to(keyboard.safe_rect(ctx))
    ///   .show(ctx, |ui| {
    ///      ui.label("it is a window");
    ///   });
    /// # });
    /// ```
    pub fn safe_rect(&self, ctx: &Context) -> Rect {
        let screen_rect = ctx.screen_rect();

        if let Some(last_rect) = self.last_rect {
            Rect::from_min_max(
                screen_rect.min,
                screen_rect.max - vec2(0., last_rect.height()),
            )
        } else {
            screen_rect
        }
    }

    /// Shows the virtual keyboard if needed.
    pub fn show(&mut self, ctx: &Context) {
        self.remember_input_widget(ctx);

        if self.keyboard_input_needed(ctx) {
            let keys = self.keyboard_layout.get_keys(self.upper, self.special);

            let response = Window::new("Keyboard")
                .frame(Frame::NONE.fill(ctx.style().visuals.extreme_bg_color))
                .collapsible(false)
                .resizable(false)
                .title_bar(false)
                .anchor(Align2::CENTER_BOTTOM, [0., 0.])
                .fixed_size(vec2(ctx.available_rect().width(), 0.))
                .order(Order::Foreground)
                .show(ctx, |ui| {
                    // We do not want any spacing between the keys.
                    ui.style_mut().spacing.item_spacing = Vec2::ZERO;

                    let widest_row = keys.iter().map(|row| row.iter().map(|key| key.width_relative()).sum::<f32>() + (row.len() as f32 + 1.0) * SPACE_BETWEEN_KEYS).reduce(f32::max).unwrap_or(0.0);
                    let available_height = ctx.available_rect().height();
                    let available_width = ui.available_width();
                    // Spacing between buttons = width of button * SPACE_BETWEEN_KEYS
                    let rows_count = keys.len() as f32;
                    let button_height = available_height / 3.0 / ((rows_count - 1.0) * SPACE_BETWEEN_KEYS + rows_count);
                    let vertical_space = button_height * SPACE_BETWEEN_KEYS;
                    // Spacing between buttons = width of button * SPACE_BETWEEN_KEYS
                    // Widest row should have `space, button, space, button, ..., button, space` -> n+1 spaces, n buttons -> (n+1)*SPACE_BETWEEN_KEYS+n buttons widths = available width
                    let button_width = available_width / widest_row;
                    let horizontal_space = button_width * SPACE_BETWEEN_KEYS;

                    ui.add_space(vertical_space);
                    self.clipboard_key(ui, horizontal_space, vertical_space);

                    for row in keys.iter() {
                        if row.is_empty() {
                            continue;
                        }
                        let row_buttons_width = row.iter().map(|key| key.width_relative()).sum::<f32>();
                        let row_len = row.len() as f32;
                        let row_total_width = row_buttons_width*button_width + (row_len + 1.0) * horizontal_space;
                        let row_total_relative_width = row_total_width / button_width;
                        let space_buttons_count = row.iter().filter(|key| matches!(key, Key::Space)).count();
                        let space_relative_width = if space_buttons_count == 0 {
                            0.0
                        } else {
                            (widest_row - row_total_relative_width) / (space_buttons_count as f32)
                        };
                        let edge_space = if space_buttons_count == 0 {
                            (available_width - row_total_width) / 2.0 + horizontal_space
                        } else {
                            horizontal_space
                        };
                        ui.horizontal(|ui| {
                            ui.add_space(edge_space);
                            for (i, key) in row.iter().enumerate() {
                                match key {
                                    Key::Text(text) => self.text_key(ui, text, Some(Vec2::new(button_width * key.width_relative(), button_height))),
                                    Key::Backspace => self.backspace_key(ui, Some(Vec2::new(button_width * key.width_relative(), button_height))),
                                    Key::Upper => self.upper_layout_key(ui, Some(Vec2::new(button_width * key.width_relative(), button_height))),
                                    Key::Space => self.text_key(ui, " ", Some(Vec2::new(button_width * space_relative_width, button_height))),
                                    Key::Special => self.special_layout_key(ui, Some(Vec2::new(button_width * key.width_relative(), button_height)))
                                }
                                if i + 1 < row.len() {
                                    ui.add_space(horizontal_space);
                                }
                            }
                            ui.add_space(horizontal_space);
                        });
                        ui.add_space(vertical_space);
                    }
                });

            if let Some(response) = response {
                self.last_rect = Some(response.response.rect);

                if response.response.contains_pointer() {
                    // Make sure Egui still thinks that we need the keyboard in the next frame.
                    self.focus_back_to_input_widget(ctx);
                }
            }

            // Prevent native keyboard from showing up.
            ctx.output_mut(|output| {
                output.ime = None;
            });
        } else {
            self.last_rect = None;
        }
    }

    fn clipboard_key(&mut self, ui: &mut Ui, horizontal_space: f32, vertical_space: f32) {
        if let Some(text) = clipboard::get_text() {
            ui.horizontal(|ui| {
                ui.add_space(horizontal_space);
                if ui.add(button(trim_text(&text, 20), None)).clicked() {
                    let event = Event::Text(text.to_string());
                    self.events.push_back(event);
                    self.focus_back_to_input_widget(ui.ctx());
                }
            });
            ui.add_space(vertical_space);
        }
    }

    /// Remember which widget had focus before the keyboard was shown.
    fn remember_input_widget(&mut self, ctx: &Context) {
        if ctx.wants_keyboard_input() {
            self.input_widget = ctx.memory(|memory| memory.focused());
        }
    }

    /// Focus back to the previously focused widget.
    fn focus_back_to_input_widget(&mut self, ctx: &Context) {
        if let Some(focus) = self.input_widget {
            ctx.memory_mut(|memory| memory.request_focus(focus));
        }
    }

    fn key(&mut self, ui: &mut Ui, text: &str, event: Event, button_size: Option<Vec2>) {
        let button = heading_button(text, button_size);
        let clicked = if let Some(size) = button_size {
            ui.add_sized(size, button).clicked()
        } else {
            ui.add(button).clicked()
        };
        if clicked  {
            self.events.push_back(event);
            self.focus_back_to_input_widget(ui.ctx());
        }
    }

    fn upper_layout_key(&mut self, ui: &mut Ui, button_size: Option<Vec2>) {
        let text = if self.upper {
            &self.shift_characters[0].to_string()
        } else {
            &self.shift_characters[1].to_string()
        };
        let button = heading_button(text, button_size);
        let clicked = if let Some(size) = button_size {
            ui.add_sized(size, button).clicked()
        } else {
            ui.add(button).clicked()
        };
        if clicked {
            self.upper = !self.upper;
            self.focus_back_to_input_widget(ui.ctx());
        }
    }

    fn special_layout_key(&mut self, ui: &mut Ui, button_size: Option<Vec2>) {
        let text = if self.special {
            "ABC"
        } else {
            "!#1"
        };
        let button = heading_button(text, button_size);
        let clicked = if let Some(size) = button_size {
            ui.add_sized(size, button).clicked()
        } else {
            ui.add(button).clicked()
        };
        if clicked {
            self.special = !self.special;
            self.focus_back_to_input_widget(ui.ctx());
        }
    }

    fn backspace_key(&mut self, ui: &mut Ui, button_size: Option<Vec2>) {
        self.key(
            ui,
            &self.backspace_character.to_string(),
            Event::Key {
                key: egui::Key::Backspace,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
                physical_key: None,
            },
            button_size
        );
    }

    fn text_key(&mut self, ui: &mut Ui, text: &str, button_size: Option<Vec2>) {
        self.key(ui, text, Event::Text(text.to_string()), button_size);
    }

    fn keyboard_input_needed(&mut self, ctx: &Context) -> bool {
        let needed = if ctx.wants_keyboard_input() {
            self.needed = 20;
            true
        } else {
            self.needed = self.needed.saturating_sub(1);
            self.needed > 0
        };

        if needed {
            ctx.request_repaint();
        }

        needed
    }
}

/// Trim the text to the maximum length, and add ellipsis if needed.
fn trim_text(text: &str, max_length: usize) -> String {
    let mut result = String::new();
    for (n, c) in text.chars().enumerate() {
        if n >= max_length {
            result.push('…');
            break;
        }
        result.push(c);
    }
    result
}
