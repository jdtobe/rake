//! Where the drop-down window goes. Everything here is in physical pixels.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// The drop-down rectangle for a monitor: anchored `top_inset` below the monitor's top edge (the
/// menubar), horizontally centered, `width_percent` of the monitor's width wide and
/// `height_percent` of the space below the inset tall.
pub fn window_rect(
    monitor_position: (i32, i32),
    monitor_size: (u32, u32),
    top_inset: u32,
    width_percent: u8,
    height_percent: u8,
) -> Rect {
    let (monitor_width, monitor_height) = monitor_size;
    let usable_height = monitor_height.saturating_sub(top_inset);
    let share =
        |total: u32, percent: u8| ((u64::from(total) * u64::from(percent) / 100) as u32).max(1);
    let width = share(monitor_width, width_percent);
    let height = share(usable_height, height_percent);
    let margin = monitor_width.saturating_sub(width) / 2;
    Rect {
        x: monitor_position.0 + margin as i32,
        y: monitor_position.1 + top_inset as i32,
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spans_full_width_at_top_of_monitor() {
        let r = window_rect((0, 0), (1920, 1080), 0, 100, 40);
        assert_eq!(
            r,
            Rect {
                x: 0,
                y: 0,
                width: 1920,
                height: 432
            }
        );
    }

    #[test]
    fn narrower_window_is_centered_horizontally() {
        let r = window_rect((0, 0), (1000, 800), 0, 60, 50);
        assert_eq!(
            r,
            Rect {
                x: 200,
                y: 0,
                width: 600,
                height: 400
            }
        );
    }

    #[test]
    fn width_and_height_percent_are_not_swapped() {
        let r = window_rect((0, 0), (1000, 1000), 0, 30, 70);
        assert_eq!((r.width, r.height), (300, 700));
    }

    #[test]
    fn odd_leftover_width_rounds_the_margin_down() {
        // 1001 * 50% = 500 wide, leaving 501 to split: 250 on the left.
        let r = window_rect((0, 0), (1001, 600), 0, 50, 50);
        assert_eq!((r.x, r.width), (250, 500));
    }

    #[test]
    fn uses_physical_pixels_on_a_retina_monitor() {
        // 2x display: tao reports the physical size, so the result is physical too.
        let r = window_rect((0, 0), (3024, 1964), 0, 100, 50);
        assert_eq!((r.width, r.height), (3024, 982));
    }

    #[test]
    fn follows_a_monitor_that_is_offset() {
        let right = window_rect((1920, 0), (2560, 1440), 0, 100, 25);
        assert_eq!((right.x, right.y), (1920, 0));
        let above = window_rect((-300, -1080), (1920, 1080), 0, 100, 25);
        assert_eq!((above.x, above.y), (-300, -1080));
        let centered = window_rect((1920, 0), (1000, 800), 0, 50, 50);
        assert_eq!(centered.x, 1920 + 250);
    }

    #[test]
    fn rests_below_the_menubar_inset() {
        let r = window_rect((0, 0), (1000, 800), 60, 100, 50);
        assert_eq!((r.y, r.height), (60, 370));
    }

    #[test]
    fn inset_is_added_to_the_monitor_origin() {
        let r = window_rect((1920, -200), (1000, 800), 60, 100, 100);
        assert_eq!((r.x, r.y, r.height), (1920, -140, 740));
    }

    #[test]
    fn inset_larger_than_the_monitor_still_gives_a_one_pixel_window() {
        let r = window_rect((0, 0), (1000, 50), 80, 100, 100);
        assert_eq!(r.height, 1);
    }

    #[test]
    fn full_height_covers_the_monitor() {
        assert_eq!(window_rect((0, 0), (1000, 700), 0, 100, 100).height, 700);
    }

    #[test]
    fn rounds_down_and_is_never_zero() {
        assert_eq!(window_rect((0, 0), (100, 333), 0, 100, 40).height, 133);
        assert_eq!(window_rect((0, 0), (100, 1), 0, 100, 10).height, 1);
        assert_eq!(window_rect((0, 0), (5, 100), 0, 10, 10).width, 1);
    }
}
