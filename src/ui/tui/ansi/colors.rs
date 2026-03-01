use ftui::PackedRgba;

use crate::infrastructure::config::ThemeName;
use crate::ui::tui::shared::ui_theme_for;

pub(in crate::ui::tui) fn ansi_16_color_for_theme(theme_name: ThemeName, index: u8) -> PackedRgba {
    let theme = ui_theme_for(theme_name);
    // Matches Catppuccin terminal palettes as shipped in official theme ports
    // (for example alacritty), keeping text slots readable on Latte.
    match index {
        0 => theme.surface1,
        1 => theme.red,
        2 => theme.green,
        3 => theme.yellow,
        4 => theme.blue,
        5 => theme.pink,
        6 => theme.teal,
        7 => theme.subtext1,
        8 => theme.surface2,
        9 => theme.red,
        10 => theme.green,
        11 => theme.yellow,
        12 => theme.blue,
        13 => theme.pink,
        14 => theme.teal,
        _ => theme.subtext0,
    }
}

pub(super) fn ansi_dim_foreground_for_theme(theme_name: ThemeName) -> PackedRgba {
    match theme_name {
        ThemeName::CatppuccinLatte => PackedRgba::rgb(140, 143, 161),
        ThemeName::CatppuccinFrappe => PackedRgba::rgb(131, 139, 167),
        ThemeName::CatppuccinMacchiato => PackedRgba::rgb(128, 135, 162),
        ThemeName::CatppuccinMocha => PackedRgba::rgb(127, 132, 156),
        ThemeName::Monokai => ui_theme_for(theme_name).subtext0,
    }
}

#[cfg(test)]
pub(in crate::ui::tui) fn ansi_16_color(index: u8) -> PackedRgba {
    ansi_16_color_for_theme(ThemeName::default(), index)
}

pub(super) fn ansi_256_color_for_theme(theme_name: ThemeName, index: u8) -> PackedRgba {
    if index < 16 {
        return ansi_16_color_for_theme(theme_name, index);
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
