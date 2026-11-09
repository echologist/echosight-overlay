use echosight_core::window::placement::{
    MIN_OVERLAY_HEIGHT, MIN_OVERLAY_WIDTH, Rect, default_bounds,
};
use gpui::{
    App, Bounds, WindowBackgroundAppearance, WindowBounds, WindowDecorations, WindowKind,
    WindowOptions, point, px, size,
};

pub fn window_options(cx: &App) -> WindowOptions {
    let area = cx
        .primary_display()
        .map(|d| {
            let b = d.visible_bounds();
            Rect {
                x: b.origin.x.as_f32().round() as i32,
                y: b.origin.y.as_f32().round() as i32,
                width: b.size.width.as_f32().round() as i32,
                height: b.size.height.as_f32().round() as i32,
            }
        })
        .unwrap_or_else(|| {
            log::warn!(
                "No primary display; using headless 1920x1080 placement, NOT visual evidence"
            );
            Rect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            }
        });
    let b = default_bounds(area);
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::new(
            point(px(b.x as f32), px(b.y as f32)),
            size(px(b.width as f32), px(b.height as f32)),
        ))),
        titlebar: None,
        focus: true,
        show: true,
        kind: WindowKind::PopUp,
        is_movable: true,
        is_resizable: true,
        is_minimizable: false,
        window_background: WindowBackgroundAppearance::Transparent,
        window_min_size: Some(size(
            px(MIN_OVERLAY_WIDTH as f32),
            px(MIN_OVERLAY_HEIGHT as f32),
        )),
        window_decorations: Some(WindowDecorations::Client),
        app_id: Some("echosight-development".into()),
        ..Default::default()
    }
}
