use egui::Color32;

pub struct VocabColors;

#[derive(Debug, Clone, Copy)]
pub struct ThemeColors {
    pub bg: Color32,
    pub surface: Color32,
    pub card_bg: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub border: Color32,
    pub correct: Color32,
    pub correct_light: Color32,
    pub wrong: Color32,
    pub wrong_light: Color32,
    pub star: Color32,
    pub star_light: Color32,
    pub primary: Color32,
}

impl ThemeColors {
    pub fn light() -> Self {
        Self {
            bg: Color32::from_rgb(0xF0, 0xF2, 0xF5),
            surface: Color32::from_rgb(0xF8, 0xF9, 0xFA),
            card_bg: Color32::WHITE,
            text_primary: Color32::from_rgb(0x1A, 0x1A, 0x2E),
            text_secondary: Color32::from_rgb(0x88, 0x88, 0x88),
            border: Color32::from_rgb(0xE0, 0xE0, 0xE0),
            correct: Color32::from_rgb(0x16, 0xA3, 0x4A),
            correct_light: Color32::from_rgb(0xDC, 0xFC, 0xE7),
            wrong: Color32::from_rgb(0xDC, 0x26, 0x26),
            wrong_light: Color32::from_rgb(0xFE, 0xE2, 0xE2),
            star: Color32::from_rgb(0xFF, 0xB8, 0x00),
            star_light: Color32::from_rgb(0xFF, 0xF8, 0xE1),
            primary: Color32::from_rgb(0x1A, 0x1A, 0x2E),
        }
    }

    pub fn dark() -> Self {
        Self {
            bg: Color32::from_rgb(0x12, 0x12, 0x18),
            surface: Color32::from_rgb(0x1E, 0x1E, 0x2E),
            card_bg: Color32::from_rgb(0x2A, 0x2A, 0x3E),
            text_primary: Color32::from_rgb(0xE8, 0xE8, 0xF0),
            text_secondary: Color32::from_rgb(0x9E, 0x9E, 0xAC),
            border: Color32::from_rgb(0x3A, 0x3A, 0x4E),
            correct: Color32::from_rgb(0x4C, 0xE8, 0x7C),
            correct_light: Color32::from_rgb(0x1A, 0x3A, 0x2A),
            wrong: Color32::from_rgb(0xFF, 0x55, 0x55),
            wrong_light: Color32::from_rgb(0x3A, 0x1A, 0x1A),
            star: Color32::from_rgb(0xFF, 0xB8, 0x00),
            star_light: Color32::from_rgb(0x3A, 0x2E, 0x0A),
            primary: Color32::from_rgb(0xBB, 0x86, 0xFC),
        }
    }
}

impl VocabColors {
    // Light theme colors (matching the original app)
    pub fn dark_bg() -> Color32 { Color32::from_rgb(0x1A, 0x1A, 0x2E) }
    pub fn light_bg() -> Color32 { Color32::from_rgb(0xF0, 0xF2, 0xF5) }
    pub fn white() -> Color32 { Color32::WHITE }
    pub fn correct() -> Color32 { Color32::from_rgb(0x16, 0xA3, 0x4A) }
    pub fn correct_light() -> Color32 { Color32::from_rgb(0xDC, 0xFC, 0xE7) }
    pub fn wrong() -> Color32 { Color32::from_rgb(0xDC, 0x26, 0x26) }
    pub fn wrong_light() -> Color32 { Color32::from_rgb(0xFE, 0xE2, 0xE2) }
    pub fn text_primary() -> Color32 { Color32::from_rgb(0x1A, 0x1A, 0x2E) }
    pub fn text_secondary() -> Color32 { Color32::from_rgb(0x88, 0x88, 0x88) }
    pub fn border() -> Color32 { Color32::from_rgb(0xE0, 0xE0, 0xE0) }
    pub fn star() -> Color32 { Color32::from_rgb(0xFF, 0xB8, 0x00) }
    pub fn star_light() -> Color32 { Color32::from_rgb(0xFF, 0xF8, 0xE1) }
    pub fn surface_light() -> Color32 { Color32::from_rgb(0xF8, 0xF9, 0xFA) }
    pub fn card_bg() -> Color32 { Color32::WHITE }
    pub fn primary() -> Color32 { Color32::from_rgb(0x1A, 0x1A, 0x2E) }
    pub fn primary_container() -> Color32 { Color32::from_rgb(0xF0, 0xF2, 0xF5) }

    // Dark theme colors (kept for backward compat)
    pub fn dark_bg_dark() -> Color32 { Color32::from_rgb(0x12, 0x12, 0x18) }
    pub fn dark_surface() -> Color32 { Color32::from_rgb(0x1E, 0x1E, 0x2E) }
    pub fn dark_card() -> Color32 { Color32::from_rgb(0x2A, 0x2A, 0x3E) }
    pub fn dark_text_primary() -> Color32 { Color32::from_rgb(0xE8, 0xE8, 0xF0) }
    pub fn dark_text_secondary() -> Color32 { Color32::from_rgb(0x9E, 0x9E, 0xAC) }
    pub fn dark_border() -> Color32 { Color32::from_rgb(0x3A, 0x3A, 0x4E) }

    pub fn for_theme(dark: bool) -> ThemeColors {
        if dark { ThemeColors::dark() } else { ThemeColors::light() }
    }
}
