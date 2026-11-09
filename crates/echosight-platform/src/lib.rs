//! Platform integration for the overlay. Spike binaries live under `examples/`.

/// Normalises a process-list line the way `poe2Monitor.ts` does and checks for Path of Exile.
pub fn is_path_of_exile_process(name: &str) -> bool {
    let normalized: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    normalized.contains("pathofexile")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_known_executable_names() {
        assert!(is_path_of_exile_process("PathOfExile.exe"));
        assert!(is_path_of_exile_process("PathOfExileSteam.exe"));
        assert!(is_path_of_exile_process("Path of Exile 2"));
        assert!(is_path_of_exile_process("pathofexile_x64.exe"));
        assert!(!is_path_of_exile_process("explorer.exe"));
    }
}

// Spike S6 supporting modules (audio, game monitor, single instance, tray).
pub mod audio;
pub mod game_monitor;
pub mod single_instance;
