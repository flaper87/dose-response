use crate::{
    color,
    engine::{Display, Settings, TextMetrics},
    point::Point,
    rect::Rectangle,
    state::State,
    ui::{self, Button},
};

pub enum Action {
    Fullscreen,
    Window,
    FontSize(i32),
    Back,
}

struct Layout {
    window_rect: Rectangle,
    rect: Rectangle,
    option_under_mouse: Option<Action>,
    rect_under_mouse: Option<Rectangle>,
    fullscreen_button: Button,
    window_button: Button,
    font_size_options: Vec<(i32, Button)>,
    back_button: Button,
}

pub struct Window;

impl Window {
    fn layout(&self, state: &State, _settings: &Settings, metrics: &dyn TextMetrics) -> Layout {
        let screen_padding = Point::from_i32(2);
        let window_rect = Rectangle::from_point_and_size(
            screen_padding,
            state.display_size - (screen_padding * 2),
        );

        let rect = Rectangle::new(
            window_rect.top_left() + (2, 0),
            window_rect.bottom_right() - (2, 1),
        );

        let mut option_under_mouse = None;
        let mut rect_under_mouse = None;

        let fullscreen_button = Button::new(rect.top_left() + (13, 3), "[F]ullscreen");
        let window_button = Button::new(rect.top_left() + (20, 3), "[W]indow");
        let back_button =
            Button::new(rect.bottom_left() + (0, -1), "[Esc] Back").align_center(rect.width());

        let button_rect = metrics.button_rect(&fullscreen_button);
        if button_rect.contains(state.mouse.tile_pos) {
            option_under_mouse = Some(Action::Fullscreen);
            rect_under_mouse = Some(button_rect);
        }

        let button_rect = metrics.button_rect(&window_button);
        if button_rect.contains(state.mouse.tile_pos) {
            option_under_mouse = Some(Action::Window);
            rect_under_mouse = Some(button_rect);
        }

        let button_rect = metrics.button_rect(&back_button);
        if button_rect.contains(state.mouse.tile_pos) {
            option_under_mouse = Some(Action::Back);
            rect_under_mouse = Some(button_rect);
        }

        let font_size_options = crate::engine::AVAILABLE_FONT_SIZES
            .iter()
            .enumerate()
            .map(|(index, &font_size)| {
                let window = crate::DISPLAY_SIZE * font_size;
                let button = Button::new(
                    rect.top_left() + (14, 6 + index as i32),
                    &format!(
                        "[{}] {}px ({}x{})",
                        index + 1,
                        font_size,
                        window.x,
                        window.y
                    ),
                );
                (font_size, button)
            })
            .collect::<Vec<_>>();

        for (size, button) in &font_size_options {
            let button_rect = metrics.button_rect(&button);
            if button_rect.contains(state.mouse.tile_pos) {
                option_under_mouse = Some(Action::FontSize(*size));
                rect_under_mouse = Some(button_rect);
            }
        }

        Layout {
            window_rect,
            rect,
            option_under_mouse,
            rect_under_mouse,
            fullscreen_button,
            window_button,
            font_size_options,
            back_button,
        }
    }

    pub fn render(
        &self,
        state: &State,
        settings: &Settings,
        metrics: &dyn TextMetrics,
        display: &mut Display,
    ) {
        use crate::ui::Text::*;

        let layout = self.layout(state, settings, metrics);

        display.draw_rectangle(layout.window_rect, color::window_edge);

        display.draw_rectangle(
            Rectangle::new(
                layout.window_rect.top_left() + (1, 1),
                layout.window_rect.bottom_right() - (1, 1),
            ),
            color::window_background,
        );

        let font_size = format!("Font size (current: {}px):", settings.font_size);

        let lines = vec![
            Centered("Settings"),
            Empty,
            Centered("Display:"),
            Centered("/"), // Fullscreen / Window
            Empty,
            Centered(&font_size),
            EmptySpace(crate::engine::AVAILABLE_FONT_SIZES.len() as i32),
            Empty,
            // TODO: read values from: `crate::engine::AVAILABLE_BACKENDS`
            Centered("Graphics backend:"),
            Centered("Glutin / SDL"),
            Empty,
            Empty, // Back
        ];

        ui::render_text_flow(&lines, layout.rect, metrics, display);

        if let Some(rect) = layout.rect_under_mouse {
            display.draw_rectangle(rect, color::menu_highlight);
        }

        // Highlight the active Fullscreen or Window option
        {
            let rect = if settings.fullscreen {
                metrics.button_rect(&layout.fullscreen_button)
            } else {
                metrics.button_rect(&layout.window_button)
            };
            display.draw_rectangle(rect, color::dim_background);
        }

        display.draw_button(&layout.fullscreen_button);
        display.draw_button(&layout.window_button);

        for (size, button) in &layout.font_size_options {
            // Highlight the active font size
            if *size == settings.font_size {
                let rect = metrics.button_rect(button);
                display.draw_rectangle(rect, color::dim_background);
            }
            display.draw_button(button)
        }

        display.draw_button(&layout.back_button);
    }

    pub fn hovered(
        &self,
        state: &State,
        settings: &Settings,
        metrics: &dyn TextMetrics,
    ) -> Option<Action> {
        self.layout(state, settings, metrics).option_under_mouse
    }
}
