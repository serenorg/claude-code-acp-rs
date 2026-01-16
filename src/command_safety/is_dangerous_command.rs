//! Dangerous command detection
//!
//! Determines if a command might be dangerous and should warn the user.
//!
//! Reference: vendors/codex/codex-rs/core/src/command_safety/is_dangerous_command.rs

use super::extract_command_basename;

/// Check if a command might be dangerous
///
/// Dangerous commands may cause data loss or system damage. They should
/// always require explicit user confirmation, even if "Always Allow" was
/// selected for that command type.
///
/// # Examples
/// ```ignore
/// assert!(command_might_be_dangerous("rm -rf /"));
/// assert!(command_might_be_dangerous("git reset --hard"));
/// assert!(!command_might_be_dangerous("ls -la"));
/// ```
pub fn command_might_be_dangerous(command: &str) -> bool {
    let parts: Vec<&str> = command.split_whitespace().collect();

    let Some(first) = parts.first() else {
        return false;
    };

    let cmd_name = extract_command_basename(first);

    match cmd_name {
        // rm with force flags is dangerous
        "rm" => is_dangerous_rm(&parts),

        // git reset and rm are dangerous (data loss)
        "git" => is_dangerous_git_subcommand(&parts),

        // sudo elevates privileges - always needs explicit confirmation
        "sudo" => true,

        // Permission changes are dangerous
        "chmod" | "chown" | "chgrp" => true,

        // System modification commands (including variants like mkfs.ext4)
        "mkfs" | "fdisk" | "parted" | "dd" => true,

        // Package managers (can install/remove software)
        "apt" | "apt-get" | "yum" | "dnf" | "pacman" | "brew" => true,

        // Service management
        "systemctl" | "service" => true,

        // Kill processes
        "kill" | "killall" | "pkill" => true,

        // Recursive operations with sudo-like effects
        "su" | "doas" => true,

        // Handle commands with prefixes (e.g., mkfs.ext4)
        _ => cmd_name.starts_with("mkfs."),
    }
}

/// Check if rm command is dangerous
///
/// rm is dangerous with:
/// - `-f` or `-rf` flags (force, no confirmation)
/// - `-r` with important paths
fn is_dangerous_rm(parts: &[&str]) -> bool {
    // Check for force flags
    for part in parts.iter().skip(1) {
        // Check for -f, -rf, -fr, etc.
        if part.starts_with('-') {
            let flags = part.trim_start_matches('-');
            if flags.contains('f') {
                return true;
            }
        }
    }
    false
}

/// Check if git subcommand is dangerous
fn is_dangerous_git_subcommand(parts: &[&str]) -> bool {
    matches!(
        parts.get(1).copied(),
        Some("reset" | "rm" | "clean" | "rebase" | "push" | "force-push")
    ) || parts.iter().any(|arg| *arg == "--force" || *arg == "-f")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dangerous_rm_commands() {
        assert!(command_might_be_dangerous("rm -rf /"));
        assert!(command_might_be_dangerous("rm -f file.txt"));
        assert!(command_might_be_dangerous("rm -rf /tmp/test"));
        assert!(command_might_be_dangerous("rm -fr /tmp/test"));
    }

    #[test]
    fn test_non_dangerous_rm_commands() {
        // rm without -f is not flagged as dangerous (user can still refuse)
        assert!(!command_might_be_dangerous("rm file.txt"));
        assert!(!command_might_be_dangerous("rm -r dir"));
        assert!(!command_might_be_dangerous("rm -i file.txt"));
    }

    #[test]
    fn test_dangerous_git_commands() {
        assert!(command_might_be_dangerous("git reset --hard"));
        assert!(command_might_be_dangerous("git reset HEAD~1"));
        assert!(command_might_be_dangerous("git rm file.txt"));
        assert!(command_might_be_dangerous("git clean -fd"));
        assert!(command_might_be_dangerous("git push --force"));
        assert!(command_might_be_dangerous("git rebase -i HEAD~3"));
    }

    #[test]
    fn test_non_dangerous_git_commands() {
        assert!(!command_might_be_dangerous("git status"));
        assert!(!command_might_be_dangerous("git log"));
        assert!(!command_might_be_dangerous("git diff"));
        assert!(!command_might_be_dangerous("git add file.txt"));
        assert!(!command_might_be_dangerous("git commit -m 'test'"));
    }

    #[test]
    fn test_sudo_always_dangerous() {
        assert!(command_might_be_dangerous("sudo ls"));
        assert!(command_might_be_dangerous("sudo rm file.txt"));
        assert!(command_might_be_dangerous("sudo apt install foo"));
    }

    #[test]
    fn test_permission_commands_dangerous() {
        assert!(command_might_be_dangerous("chmod 777 file"));
        assert!(command_might_be_dangerous("chown root file"));
        assert!(command_might_be_dangerous("chgrp admin file"));
    }

    #[test]
    fn test_system_commands_dangerous() {
        assert!(command_might_be_dangerous("mkfs.ext4 /dev/sda1"));
        assert!(command_might_be_dangerous("fdisk /dev/sda"));
        assert!(command_might_be_dangerous("dd if=/dev/zero of=/dev/sda"));
    }

    #[test]
    fn test_package_managers_dangerous() {
        assert!(command_might_be_dangerous("apt install foo"));
        assert!(command_might_be_dangerous("apt-get remove bar"));
        assert!(command_might_be_dangerous("brew install baz"));
        assert!(command_might_be_dangerous("pacman -S foo"));
    }

    #[test]
    fn test_service_management_dangerous() {
        assert!(command_might_be_dangerous("systemctl restart nginx"));
        assert!(command_might_be_dangerous("service apache2 stop"));
    }

    #[test]
    fn test_process_killing_dangerous() {
        assert!(command_might_be_dangerous("kill -9 1234"));
        assert!(command_might_be_dangerous("killall firefox"));
        assert!(command_might_be_dangerous("pkill -f python"));
    }

    #[test]
    fn test_privilege_escalation_dangerous() {
        assert!(command_might_be_dangerous("su -"));
        assert!(command_might_be_dangerous("doas ls"));
    }

    #[test]
    fn test_safe_commands_not_dangerous() {
        assert!(!command_might_be_dangerous("ls -la"));
        assert!(!command_might_be_dangerous("cat file.txt"));
        assert!(!command_might_be_dangerous("grep pattern file"));
        assert!(!command_might_be_dangerous("find . -name '*.rs'"));
        assert!(!command_might_be_dangerous("echo hello"));
    }

    #[test]
    fn test_full_path_dangerous_commands() {
        assert!(command_might_be_dangerous("/usr/bin/sudo ls"));
        assert!(command_might_be_dangerous("/bin/rm -rf /tmp/test"));
        assert!(command_might_be_dangerous("/usr/bin/git reset --hard"));
    }

    #[test]
    fn test_empty_command() {
        assert!(!command_might_be_dangerous(""));
        assert!(!command_might_be_dangerous("   "));
    }
}
