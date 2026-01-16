//! Command safety analysis module
//!
//! This module provides functions to determine if shell commands are safe to execute
//! automatically or potentially dangerous and require user confirmation.
//!
//! Reference: vendors/codex/codex-rs/core/src/command_safety/

mod is_dangerous_command;
mod is_safe_command;

pub use is_dangerous_command::command_might_be_dangerous;
pub use is_safe_command::is_known_safe_command;

/// Extract the basename of a command, handling full paths
///
/// Examples:
/// - `/usr/bin/find` → `find`
/// - `find` → `find`
/// - `` → ``
pub fn extract_command_basename(cmd: &str) -> &str {
    cmd.split_whitespace()
        .next()
        .and_then(|s| std::path::Path::new(s).file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_command_basename() {
        assert_eq!(extract_command_basename("find . -name '*.rs'"), "find");
        assert_eq!(extract_command_basename("/usr/bin/find ."), "find");
        assert_eq!(extract_command_basename("/usr/local/bin/git status"), "git");
        assert_eq!(extract_command_basename("ls -la"), "ls");
        assert_eq!(extract_command_basename(""), "");
    }
}
