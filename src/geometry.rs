//! Where the drop-down window goes. Everything here is in physical pixels.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// The drop-down rectangle for a monitor: full width, anchored to the top edge, and
/// `height_percent` of the monitor's height tall.
pub fn window_rect(
    monitor_position: (i32, i32),
    monitor_size: (u32, u32),
    height_percent: u8,
) -> Rect {
    let (width, monitor_height) = monitor_size;
    let height = (u64::from(monitor_height) * u64::from(height_percent) / 100) as u32;
    Rect {
        x: monitor_position.0,
        y: monitor_position.1,
        width,
        height: height.max(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spans_full_width_at_top_of_monitor() {
        let r = window_rect((0, 0), (1920, 1080), 40);
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
    fn uses_physical_pixels_on_a_retina_monitor() {
        // 2x display: tao reports the physical size, so the result is physical too.
        let r = window_rect((0, 0), (3024, 1964), 50);
        assert_eq!((r.width, r.height), (3024, 982));
    }

    #[test]
    fn follows_a_monitor_that_is_offset() {
        let right = window_rect((1920, 0), (2560, 1440), 25);
        assert_eq!((right.x, right.y), (1920, 0));
        let above = window_rect((-300, -1080), (1920, 1080), 25);
        assert_eq!((above.x, above.y), (-300, -1080));
    }

    #[test]
    fn full_height_covers_the_monitor() {
        assert_eq!(window_rect((0, 0), (1000, 700), 100).height, 700);
    }

    #[test]
    fn rounds_down_and_is_never_zero() {
        assert_eq!(window_rect((0, 0), (100, 333), 40).height, 133);
        assert_eq!(window_rect((0, 0), (100, 1), 10).height, 1);
    }
}
