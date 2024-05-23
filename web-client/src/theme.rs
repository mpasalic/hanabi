pub fn get_default_theme() {
    use egui::ecolor::*;
    use egui::epaint::{Rounding, Shadow, Stroke};

    use egui::{
        ecolor::*, emath::*, ComboBox, CursorIcon, FontFamily, FontId, Margin, Response, RichText,
        WidgetText,
    };

    let mut visuals = Visuals {
        dark_mode: true,
        override_text_color: None,
        widgets: Widgets {
            noninteractive: WidgetVisuals {
                weak_bg_fill: Color32::from_gray(0),
                bg_fill: Color32::from_gray(0),
                bg_stroke: Stroke::new(1.0, Color32::from_gray(60)), // separators, indentation lines
                fg_stroke: Stroke::new(1.0, Color32::from_gray(140)), // normal text color
                rounding: Rounding::same(2.0),
                expansion: 0.0,
            },
            inactive: WidgetVisuals {
                weak_bg_fill: Color32::from_gray(0), // button background
                bg_fill: Color32::from_gray(0),      // checkbox background
                bg_stroke: Default::default(),
                fg_stroke: Stroke::new(1.0, Color32::from_gray(180)), // button text
                rounding: Rounding::same(2.0),
                expansion: 0.0,
            },
            hovered: WidgetVisuals {
                weak_bg_fill: Color32::from_gray(0),
                bg_fill: Color32::from_gray(0),
                bg_stroke: Stroke::new(1.0, Color32::from_gray(150)), // e.g. hover over window edge or button
                fg_stroke: Stroke::new(1.5, Color32::from_gray(240)),
                rounding: Rounding::same(3.0),
                expansion: 1.0,
            },
            active: WidgetVisuals {
                weak_bg_fill: Color32::from_gray(0),
                bg_fill: Color32::from_gray(0),
                bg_stroke: Stroke::new(1.0, Color32::WHITE),
                fg_stroke: Stroke::new(2.0, Color32::WHITE),
                rounding: Rounding::same(2.0),
                expansion: 1.0,
            },
            open: WidgetVisuals {
                weak_bg_fill: Color32::from_gray(0),
                bg_fill: Color32::from_gray(0),
                bg_stroke: Stroke::new(1.0, Color32::from_gray(60)),
                fg_stroke: Stroke::new(1.0, Color32::from_gray(210)),
                rounding: Rounding::same(2.0),
                expansion: 0.0,
            },
        },
        selection: Selection {
            bg_fill: Color32::from_gray(0),
            stroke: Stroke::new(1.0, Color32::from_gray(0)),
        },
        hyperlink_color: Color32::from_rgb(0, 0, 0),
        faint_bg_color: Color32::from_rgb(0, 0, 0), // visible, but barely so
        extreme_bg_color: Color32::from_rgb(0, 0, 0),
        code_bg_color: Color32::from_rgb(0, 0, 0),
        warn_fg_color: Color32::from_rgb(0, 0, 0),
        error_fg_color: Color32::from_rgb(0, 0, 0),

        window_rounding: Rounding::same(6.0),
        window_shadow: Shadow {
            offset: vec2(10.0, 20.0),
            blur: 15.0,
            spread: 0.0,
            color: Color32::from_black_alpha(96),
        },
        window_fill: Color32::from_gray(0),
        window_stroke: Stroke::new(1.0, Color32::from_gray(60)),
        window_highlight_topmost: true,

        menu_rounding: Rounding::same(6.0),

        panel_fill: Color32::from_gray(0),

        popup_shadow: Shadow {
            offset: vec2(6.0, 10.0),
            blur: 8.0,
            spread: 0.0,
            color: Color32::from_black_alpha(96),
        },

        resize_corner_size: 12.0,

        text_cursor: Stroke::new(2.0, Color32::from_rgb(192, 222, 255)),
        text_cursor_preview: false,

        clip_rect_margin: 3.0, // should be at least half the size of the widest frame stroke + max WidgetVisuals::expansion
        button_frame: true,
        collapsing_header_frame: false,
        indent_has_left_vline: true,

        striped: false,

        slider_trailing_fill: false,
        handle_shape: egui::Visuals::dark().handle_shape,

        interact_cursor: None,

        image_loading_spinners: true,

        numeric_color_space: egui::Visuals::dark().numeric_color_space,
    };
}
