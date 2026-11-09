//! Portable S5 harness. Uses S1 window options; every non-system asset is embedded.
mod common;

use echosight_ui::{components::text_input, spikes::InputSpike};
use gpui::{App, AppContext};

fn main() {
    if std::env::args().any(|arg| arg == "--licenses") {
        println!(
            "{}\n{}",
            include_str!("../../echosight-ui/THIRD_PARTY.md"),
            include_str!("../../echosight-ui/LICENSE-APACHE")
        );
        return;
    }
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!("S5 input starting ({})", common::gpui_rev());
    gpui_platform::application().run(|cx: &mut App| {
        text_input::init(cx);
        cx.open_window(common::window_options(cx), |window, cx| {
            cx.new(|cx| InputSpike::new(window, cx))
        })
        .expect("open S5 input window");
        log::info!("S5 window opened; native input/IME still requires manual verification");
    });
}
