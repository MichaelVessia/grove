const SIDEBAR_RATIO_MIN_PCT: u16 = 10;
const SIDEBAR_RATIO_MAX_PCT: u16 = 60;

pub fn clamp_sidebar_ratio(ratio_pct: u16) -> u16 {
    ratio_pct.clamp(SIDEBAR_RATIO_MIN_PCT, SIDEBAR_RATIO_MAX_PCT)
}

pub fn ratio_from_drag(total_width: u16, drag_x: u16) -> u16 {
    if total_width == 0 {
        return SIDEBAR_RATIO_MIN_PCT;
    }

    let ratio_u32 = u32::from(drag_x).saturating_mul(100) / u32::from(total_width);
    let ratio = u16::try_from(ratio_u32).unwrap_or(u16::MAX);
    clamp_sidebar_ratio(ratio)
}

#[cfg(test)]
mod tests {
    use super::{clamp_sidebar_ratio, ratio_from_drag};

    #[test]
    fn drag_ratio_is_clamped_between_ten_and_sixty_percent() {
        assert_eq!(ratio_from_drag(100, 5), 10);
        assert_eq!(ratio_from_drag(100, 50), 50);
        assert_eq!(ratio_from_drag(100, 90), 60);
    }

    #[test]
    fn clamp_sidebar_ratio_bounds_values() {
        assert_eq!(clamp_sidebar_ratio(0), 10);
        assert_eq!(clamp_sidebar_ratio(33), 33);
        assert_eq!(clamp_sidebar_ratio(100), 60);
    }
}
