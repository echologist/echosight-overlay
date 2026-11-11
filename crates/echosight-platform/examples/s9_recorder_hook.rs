//! Isolated Windows raw-snapshot experiment. Not a production recorder/DOM decoder.
#[path = "recorder_hook_support/decode.rs"]
#[cfg_attr(not(windows), allow(dead_code))]
mod decode;
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
