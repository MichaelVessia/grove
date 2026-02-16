use ftui::PackedRgba;

pub(in crate::ui::tui) fn ansi_16_color(index: u8) -> PackedRgba {
    match index {
        0 => PackedRgba::rgb(0, 0, 0),
        1 => PackedRgba::rgb(205, 49, 49),
        2 => PackedRgba::rgb(13, 188, 121),
        3 => PackedRgba::rgb(229, 229, 16),
        4 => PackedRgba::rgb(36, 114, 200),
        5 => PackedRgba::rgb(188, 63, 188),
        6 => PackedRgba::rgb(17, 168, 205),
        7 => PackedRgba::rgb(229, 229, 229),
        8 => PackedRgba::rgb(102, 102, 102),
        9 => PackedRgba::rgb(241, 76, 76),
        10 => PackedRgba::rgb(35, 209, 139),
        11 => PackedRgba::rgb(245, 245, 67),
        12 => PackedRgba::rgb(59, 142, 234),
        13 => PackedRgba::rgb(214, 112, 214),
        14 => PackedRgba::rgb(41, 184, 219),
        _ => PackedRgba::rgb(255, 255, 255),
    }
}

pub(super) fn ansi_256_color(index: u8) -> PackedRgba {
    if index < 16 {
        return ansi_16_color(index);
    }

    if index <= 231 {
        let value = usize::from(index - 16);
        let r = value / 36;
        let g = (value % 36) / 6;
        let b = value % 6;
        let table = [0u8, 95, 135, 175, 215, 255];
        return PackedRgba::rgb(table[r], table[g], table[b]);
    }

    let gray = 8u8.saturating_add((index - 232).saturating_mul(10));
    PackedRgba::rgb(gray, gray, gray)
}
