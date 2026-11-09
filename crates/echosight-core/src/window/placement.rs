//! Overlay window placement. Port of `src/main/overlay/windowPlacement.ts`.

pub const DEFAULT_OVERLAY_WIDTH: i32 = 500;
pub const DEFAULT_OVERLAY_HEIGHT: i32 = 700;
pub const DEFAULT_MARGIN: i32 = 20;
pub const MIN_OVERLAY_WIDTH: i32 = 450;
pub const MIN_OVERLAY_HEIGHT: i32 = 600;

/// A rectangle in logical screen pixels (work area of a display, or window bounds).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Top-right anchor inset by `margin`, never pushed left of the work area.
/// Mirrors `getOverlayAnchorPosition(workArea, { width, margin })`.
pub fn anchor_position(work_area: Rect, width: i32, margin: i32) -> (i32, i32) {
    let x = work_area.x + (work_area.width - width - margin).max(0);
    let y = work_area.y + margin;
    (x, y)
}

/// Default window bounds. Mirrors `getOverlayWindowBounds(workArea)`.
pub fn default_bounds(work_area: Rect) -> Rect {
    let (x, y) = anchor_position(work_area, DEFAULT_OVERLAY_WIDTH, DEFAULT_MARGIN);
    Rect {
        x,
        y,
        width: DEFAULT_OVERLAY_WIDTH,
        height: DEFAULT_OVERLAY_HEIGHT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Examples asserted by tests/unit/main/windowPlacement.test.ts.
    #[test]
    fn default_bounds_on_1080p_work_area() {
        let wa = Rect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1040,
        };
        assert_eq!(
            default_bounds(wa),
            Rect {
                x: 1400,
                y: 20,
                width: 500,
                height: 700
            }
        );
    }

    #[test]
    fn anchor_on_secondary_display_left_of_primary() {
        let wa = Rect {
            x: -1920,
            y: 40,
            width: 1920,
            height: 1040,
        };
        assert_eq!(anchor_position(wa, 500, 20), (-520, 60));
    }

    #[test]
    fn anchor_never_moves_left_of_narrow_work_area() {
        let wa = Rect {
            x: 100,
            y: 30,
            width: 420,
            height: 800,
        };
        assert_eq!(anchor_position(wa, 500, 20), (100, 50));
    }
}
