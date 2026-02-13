use ftui::render::cell::StyleFlags;
use ftui::{Frame, PackedRgba};

pub fn row_text(frame: &Frame, y: u16, x_start: u16, x_end: u16) -> String {
    (x_start..x_end)
        .filter_map(|x| {
            frame
                .buffer
                .get(x, y)
                .and_then(|cell| cell.content.as_char())
        })
        .collect::<String>()
        .trim_end()
        .to_string()
}

pub fn find_row_containing(frame: &Frame, text: &str, x_start: u16, x_end: u16) -> Option<u16> {
    (0..frame.height()).find(|&y| row_text(frame, y, x_start, x_end).contains(text))
}

pub fn assert_cell_style(frame: &Frame, x: u16, y: u16, flags: StyleFlags) {
    let Some(cell) = frame.buffer.get(x, y) else {
        panic!("cell ({x},{y}) should be in bounds");
    };
    assert!(
        cell.attrs.flags().contains(flags),
        "cell ({x},{y}) expected {flags:?}, got {:?}",
        cell.attrs.flags()
    );
}

pub fn find_cell_with_char(
    frame: &Frame,
    y: u16,
    x_start: u16,
    x_end: u16,
    ch: char,
) -> Option<u16> {
    (x_start..x_end).find(|&x| {
        frame
            .buffer
            .get(x, y)
            .and_then(|cell| cell.content.as_char())
            == Some(ch)
    })
}

pub fn assert_row_fg(frame: &Frame, y: u16, x_start: u16, x_end: u16, expected_fg: PackedRgba) {
    for x in x_start..x_end {
        let Some(cell) = frame.buffer.get(x, y) else {
            continue;
        };
        let is_visible = cell
            .content
            .as_char()
            .is_some_and(|value| !value.is_whitespace());
        if is_visible {
            assert_eq!(cell.fg, expected_fg, "cell ({x},{y}) has unexpected fg");
        }
    }
}
