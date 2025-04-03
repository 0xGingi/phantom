use tui::style::Color;

pub fn parse_color(color_str: &str) -> Color {
    if let Ok(rgb) = u32::from_str_radix(&color_str[1..], 16) {
        Color::Rgb(
            ((rgb >> 16) & 0xFF) as u8,
            ((rgb >> 8) & 0xFF) as u8,
            (rgb & 0xFF) as u8,
        )
    } else {
        Color::Reset
    }
} 