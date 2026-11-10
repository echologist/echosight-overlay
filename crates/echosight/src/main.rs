use echosight_ui::{app::TaskApp, components::text_input};
use gpui::{App, AppContext};
use std::path::PathBuf;

fn data_dir(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<PathBuf, String> {
    let mut args = args.into_iter();
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--data-dir")) {
        return Err("Usage: echosight --data-dir <explicit-development-directory>".into());
    }
    let dir = args
        .next()
        .filter(|s| !s.is_empty())
        .ok_or("--data-dir requires a directory")?;
    if args.next().is_some() {
        return Err("Unexpected argument".into());
    }
    Ok(dir.into())
}

fn main() {
    if std::env::args().any(|arg| arg == "--licenses") {
        println!(
            "{}\n{}",
            include_str!("../../echosight-ui/THIRD_PARTY.md"),
            include_str!("../../echosight-ui/LICENSE-APACHE")
        );
        return;
    }
    let dir = match data_dir(std::env::args_os().skip(1)) {
        Ok(dir) => dir,
        Err(error) => {
            eprintln!("{error}\nLegacy application directory is unverified. No data was accessed.");
            std::process::exit(2);
        }
    };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!(
        "Development task app; explicit data directory: {}",
        dir.display()
    );
    gpui_platform::application().run(move |cx: &mut App| {
        text_input::init(cx);
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        cx.open_window(echosight_ui::app::window_options(cx), |window, cx| {
            cx.new(|cx| TaskApp::new(dir, window, cx))
        })
        .expect("open Echosight task window");
        log::info!("Task window opened; native visual/IME acceptance remains pending");
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_directory_is_mandatory() {
        assert!(data_dir([]).is_err());
        assert!(data_dir(["--data-dir".into()]).is_err());
        assert!(data_dir(["--data-dir".into(), "".into()]).is_err());
        assert!(data_dir(["--data-dir".into(), "dev".into(), "extra".into()]).is_err());
        assert_eq!(
            data_dir(["--data-dir".into(), "dev tasks".into()]).unwrap(),
            PathBuf::from("dev tasks")
        );
    }
}
