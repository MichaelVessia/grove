mod colors;
mod parser;

#[cfg(test)]
pub(super) use colors::ansi_16_color;
#[cfg(test)]
pub(super) use parser::ansi_lines_to_styled_lines;
pub(super) use parser::ansi_lines_to_styled_lines_for_theme;
