//! Slash command support
//!
//! This module provides predefined slash commands that are sent to clients
//! via the ACP protocol's `available_commands_update` notification.

use sacp::schema::{AvailableCommand, AvailableCommandInput, UnstructuredCommandInput};

/// Cached regex for matching MCP command format
/// Pattern: /mcp:server:name [args]
static MCP_COMMAND_REGEX: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
    regex::Regex::new(r"^/mcp:([^:\s]+):(\S+)(\s+.*)?$").unwrap()
});

/// Predefined slash commands
///
/// These commands are sent to the client when a session starts.
/// The client can display them to users for quick access.
pub fn get_predefined_commands() -> Vec<AvailableCommand> {
    vec![
        AvailableCommand::new("compact", "Compact conversation with optional focus instructions")
            .input(Some(AvailableCommandInput::Unstructured(
                UnstructuredCommandInput::new("[instructions]"),
            ))),
        AvailableCommand::new("init", "Initialize project with CLAUDE.md guide")
            .input(Some(AvailableCommandInput::Unstructured(
                UnstructuredCommandInput::new(""),
            ))),
        AvailableCommand::new("review", "Request code review")
            .input(Some(AvailableCommandInput::Unstructured(
                UnstructuredCommandInput::new("[scope or file]"),
            ))),
    ]
}

/// Transform MCP command input format
///
/// Converts user input from ACP format to SDK format:
/// - ACP: "/mcp:server:name args"
/// - SDK: "/server:name (MCP) args"
pub fn transform_mcp_command_input(text: &str) -> String {
    // Match /mcp:server:name format using cached regex
    if let Some(caps) = MCP_COMMAND_REGEX.captures(text) {
        let server = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let command = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let args = caps.get(3).map(|m| m.as_str()).unwrap_or("");
        format!("/{}:{} (MCP){}", server, command, args)
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_mcp_command_input() {
        // Standard MCP command with args
        assert_eq!(
            transform_mcp_command_input("/mcp:server:cmd some args"),
            "/server:cmd (MCP) some args"
        );
        // Regular command (no transformation)
        assert_eq!(
            transform_mcp_command_input("/compact"),
            "/compact"
        );
        // MCP command without args
        assert_eq!(
            transform_mcp_command_input("/mcp:test:run"),
            "/test:run (MCP)"
        );
    }

    #[test]
    fn test_predefined_commands() {
        let commands = get_predefined_commands();
        assert!(!commands.is_empty());
        assert!(commands.iter().any(|c| c.name == "compact"));
        assert!(commands.iter().any(|c| c.name == "init"));
        assert!(commands.iter().any(|c| c.name == "review"));
    }

    #[test]
    fn test_command_descriptions() {
        let commands = get_predefined_commands();
        for cmd in commands {
            assert!(
                !cmd.description.is_empty(),
                "Command {} should have description",
                cmd.name
            );
        }
    }

    // Edge case tests

    #[test]
    fn test_empty_string() {
        assert_eq!(transform_mcp_command_input(""), "");
    }

    #[test]
    fn test_non_slash_command() {
        assert_eq!(transform_mcp_command_input("hello world"), "hello world");
    }

    #[test]
    fn test_regular_slash_command() {
        assert_eq!(transform_mcp_command_input("/commit"), "/commit");
        assert_eq!(transform_mcp_command_input("/review file.rs"), "/review file.rs");
    }

    #[test]
    fn test_mcp_command_without_command_name() {
        // Incomplete MCP format - should not match
        assert_eq!(transform_mcp_command_input("/mcp:server"), "/mcp:server");
    }

    #[test]
    fn test_mcp_command_with_special_chars() {
        assert_eq!(
            transform_mcp_command_input("/mcp:my-server:run-tests --verbose"),
            "/my-server:run-tests (MCP) --verbose"
        );
    }

    #[test]
    fn test_command_count() {
        let commands = get_predefined_commands();
        assert_eq!(commands.len(), 3);
    }
}
