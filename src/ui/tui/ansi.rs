mod colors;
mod parser;

#[cfg(test)]
pub(super) use colors::ansi_16_color;
pub(super) use parser::ansi_line_to_styled_line;
