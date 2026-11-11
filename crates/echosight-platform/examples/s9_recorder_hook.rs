//! Isolated Windows raw-snapshot experiment. Not a production recorder/DOM decoder.
#[cfg(any(windows, test))]
#[path = "recorder_hook_support/canaries.rs"]
mod canaries;

#[path = "recorder_hook_support/decode.rs"]
#[cfg_attr(not(windows), allow(dead_code))]
mod decode;
#[allow(unexpected_cfgs)]
#[cfg(test)]
mod gpui_timeout_tests {
    // CLI-only test-support feature and cfg; no manifest changes needed.
    #[cfg(s9_gpui_timeout_tests)]
    include!("recorder_hook_support/gpui_timeout_tests.rs");
}
#[cfg(windows)]
#[path = "recorder_hook_support/native.rs"]
mod native;
#[path = "recorder_hook_support/policy.rs"]
#[cfg_attr(not(windows), allow(dead_code))]
mod policy;
#[cfg(windows)]
#[path = "recorder_hook_support/probe.rs"]
mod probe;

#[cfg(windows)]
fn main() {
    probe::run();
}

#[cfg(not(windows))]
fn main() {
    eprintln!(
        "S9 unsupported: requires native Windows. Pure policy tests are not native evidence."
    );
    std::process::exit(2);
}
