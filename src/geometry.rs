//! Where the drop-down window goes. Everything here is in physical pixels.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// The drop-down rectangle for a monitor: anchored to the top edge, horizontally centered,
/// `width_percent` of the monitor's width wide and `height_percent` of its height tall.
pub fn window_rect(
    monitor_position: (i32, i32),
    monitor_size: (u32, u32),
    width_percent: u8,
    height_percent: u8,
) -> Rect {
    let (monitor_width, monitor_height) = monitor_size;
    let share =
        |total: u32, percent: u8| ((u64::from(total) * u64::from(percent) / 100) as u32).max(1);
    let width = share(monitor_width, width_percent);
    let height = share(monitor_height, height_percent);
    let margin = monitor_width.saturating_sub(width) / 2;
    Rect {
        x: monitor_position.0 + margin as i32,
        y: monitor_position.1,
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spans_full_width_at_top_of_monitor() {
        let r = window_rect((0, 0), (1920, 1080), 100, 40);
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
        let r = window_rect((0, 0), (1000, 800), 60, 50);
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
        let r = window_rect((0, 0), (1000, 1000), 30, 70);
        assert_eq!((r.width, r.height), (300, 700));
    }

    #[test]
    fn odd_leftover_width_rounds_the_margin_down() {
        // 1001 * 50% = 500 wide, leaving 501 to split: 250 on the left.
        let r = window_rect((0, 0), (1001, 600), 50, 50);
        assert_eq!((r.x, r.width), (250, 500));
    }

    #[test]
    fn uses_physical_pixels_on_a_retina_monitor() {
        // 2x display: tao reports the physical size, so the result is physical too.
        let r = window_rect((0, 0), (3024, 1964), 100, 50);
        assert_eq!((r.width, r.height), (3024, 982));
    }

    #[test]
    fn follows_a_monitor_that_is_offset() {
        let right = window_rect((1920, 0), (2560, 1440), 100, 25);
        assert_eq!((right.x, right.y), (1920, 0));
        let above = window_rect((-300, -1080), (1920, 1080), 100, 25);
        assert_eq!((above.x, above.y), (-300, -1080));
        let centered = window_rect((1920, 0), (1000, 800), 50, 50);
        assert_eq!(centered.x, 1920 + 250);
    }

    #[test]
    fn full_height_covers_the_monitor() {
        assert_eq!(window_rect((0, 0), (1000, 700), 100, 100).height, 700);
    }

    #[test]
    fn rounds_down_and_is_never_zero() {
        assert_eq!(window_rect((0, 0), (100, 333), 100, 40).height, 133);
        assert_eq!(window_rect((0, 0), (100, 1), 100, 10).height, 1);
        assert_eq!(window_rect((0, 0), (5, 100), 10, 10).width, 1);
    }
}
