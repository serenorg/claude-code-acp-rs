//! PreToolUse hook implementation
//!
//! Checks permissions using SettingsManager before tool execution.
//! For "Ask" decisions, sends permission request directly (has correct tool_use_id).

use std::sync::{Arc, OnceLock};
use std::time::Instant;

use claude_code_agent_sdk::{
    HookCallback, HookContext, HookInput, HookJsonOutput, HookSpecificOutput,
    PreToolUseHookSpecificOutput, SyncHookJsonOutput,
};
use dashmap::DashMap;
use futures::future::BoxFuture;
use sacp::{JrConnectionCx, link::AgentToClient};
use tokio::sync::RwLock;
use tracing::Instrument;

use crate::session::PermissionMode;
use crate::settings::PermissionChecker;

/// Creates a PreToolUse hook that checks permissions using settings rules and permission mode.
///
/// This hook runs before the SDK's built-in permission rules, allowing us to enforce
/// our own permission settings for ACP-prefixed tools.
///
/// # Permission Handling
///
/// - **Allow**: Returns with `permission_decision: "allow"` - tool executes immediately
/// - **Deny**: Returns with `permission_decision: "deny"` - tool execution is blocked
/// - **Ask**: Returns with `permission_decision: "ask"` - SDK triggers permission request flow
///
/// # Permission Mode Integration
///
/// The hook respects the session's permission mode:
/// - **BypassPermissions/AcceptEdits**: Allows all tools without checking rules
///   (AcceptEdits behaves like BypassPermissions for root compatibility)
/// - **Plan**: Blocks write operations (Edit, Write, Bash, NotebookEdit)
/// - **Default/DontAsk**: Checks settings rules and mode-based auto-approval
///
/// # Architecture
///
/// The hook and `can_use_tool` callback work together:
/// 1. **Hook** (this file): Makes quick decisions based on static rules (allow/deny/ask)
/// 2. **`can_use_tool` callback** (`can_use_tool.rs`): Checks cached permission results
///
/// For "Ask" decisions, the hook sends permission request directly (using the correct tool_use_id),
/// then caches the result. The `can_use_tool` callback checks this cache and returns immediately.
///
/// # Arguments
///
/// * `connection_cx_lock` - Connection for sending permission requests
/// * `session_id` - Session ID for permission requests
/// * `permission_checker` - Optional permission checker for settings-based rules
/// * `permission_mode` - Shared permission mode that can be updated at runtime
/// * `permission_cache` - Cache for storing permission results (for can_use_tool callback)
/// * `tool_use_id_cache` - Cache for storing tool_use_id (for can_use_tool callback)
///
/// # Returns
///
/// A hook callback that can be used with ClaudeAgentOptions
pub fn create_pre_tool_use_hook(
    connection_cx_lock: Arc<OnceLock<JrConnectionCx<AgentToClient>>>,
    session_id: String,
    permission_checker: Option<Arc<RwLock<PermissionChecker>>>,
    permission_mode: Arc<RwLock<PermissionMode>>,
    permission_cache: Arc<DashMap<String, bool>>,
    tool_use_id_cache: Arc<DashMap<String, String>>,
) -> HookCallback {
    Arc::new(
        move |input: HookInput, tool_use_id: Option<String>, _context: HookContext| {
            // These parameters are kept for API compatibility but no longer used here.
            // Permission requests are now handled by can_use_tool callback.
            let _connection_cx_lock = Arc::clone(&connection_cx_lock);
            let permission_checker = permission_checker.clone();
            let permission_mode = permission_mode.clone();
            let _session_id = session_id.clone();
            let _permission_cache = Arc::clone(&permission_cache);
            let tool_use_id_cache = Arc::clone(&tool_use_id_cache);

            // Extract tool name early for span naming
            let (tool_name, is_pre_tool) = match &input {
                HookInput::PreToolUse(pre_tool) => (pre_tool.tool_name.clone(), true),
                _ => ("".to_string(), false),
            };

            // Create a span for this hook execution
            let span = if is_pre_tool {
                tracing::info_span!(
                    "pre_tool_use_hook",
                    tool_name = %tool_name,
                    tool_use_id = ?tool_use_id,
                    permission_decision = tracing::field::Empty,
                    permission_rule = tracing::field::Empty,
                    check_duration_us = tracing::field::Empty,
                )
            } else {
                tracing::debug_span!(
                    "pre_tool_use_hook_skip",
                    event_type = ?std::mem::discriminant(&input)
                )
            };

            Box::pin(
                async move {
                    let start_time = Instant::now();

                    // Only handle PreToolUse events
                    let (tool_name, tool_input) = match &input {
                        HookInput::PreToolUse(pre_tool) => {
                            (pre_tool.tool_name.clone(), pre_tool.tool_input.clone())
                        }
                        _ => {
                            tracing::debug!("Ignoring non-PreToolUse event");
                            return HookJsonOutput::Sync(SyncHookJsonOutput {
                                continue_: Some(true),
                                ..Default::default()
                            });
                        }
                    };

                    tracing::debug!(
                        tool_name = %tool_name,
                        tool_use_id = ?tool_use_id,
                        "PreToolUse hook triggered"
                    );

                    // Get current permission mode
                    let mode = *permission_mode.read().await;

                    // BypassPermissions and AcceptEdits modes allow everything
                    // (AcceptEdits behaves like BypassPermissions for root compatibility)
                    if matches!(
                        mode,
                        PermissionMode::BypassPermissions | PermissionMode::AcceptEdits
                    ) {
                        let elapsed = start_time.elapsed();
                        let mode_str = match mode {
                            PermissionMode::BypassPermissions => "BypassPermissions",
                            PermissionMode::AcceptEdits => "AcceptEdits",
                            _ => unreachable!(),
                        };
                        tracing::info!(
                            tool_name = %tool_name,
                            tool_use_id = ?tool_use_id,
                            mode = %mode_str,
                            elapsed_us = elapsed.as_micros(),
                            "Tool allowed by permission mode (auto-approve all)"
                        );

                        return HookJsonOutput::Sync(SyncHookJsonOutput {
                            continue_: Some(true),
                            hook_specific_output: Some(HookSpecificOutput::PreToolUse(
                                PreToolUseHookSpecificOutput {
                                    permission_decision: Some("allow".to_string()),
                                    permission_decision_reason: Some(format!(
                                        "Allowed by {} mode (auto-approve all tools)",
                                        mode_str
                                    )),
                                    updated_input: None,
                                },
                            )),
                            ..Default::default()
                        });
                    }

                    // Plan mode: block write operations
                    if mode == PermissionMode::Plan {
                        let is_write_operation = matches!(
                            tool_name.as_str(),
                            "Edit" | "Write" | "Bash" | "NotebookEdit"
                        );
                        if is_write_operation {
                            let elapsed = start_time.elapsed();
                            let reason =
                                format!("Tool {} is blocked in Plan mode (read-only)", tool_name);
                            tracing::warn!(
                                tool_name = %tool_name,
                                tool_use_id = ?tool_use_id,
                                mode = "plan",
                                elapsed_us = elapsed.as_micros(),
                                "Tool blocked by Plan mode"
                            );

                            return HookJsonOutput::Sync(SyncHookJsonOutput {
                                continue_: Some(true),
                                hook_specific_output: Some(HookSpecificOutput::PreToolUse(
                                    PreToolUseHookSpecificOutput {
                                        permission_decision: Some("deny".to_string()),
                                        permission_decision_reason: Some(reason),
                                        updated_input: None,
                                    },
                                )),
                                ..Default::default()
                            });
                        }
                        // Read operations in Plan mode: allow them
                        let elapsed = start_time.elapsed();
                        tracing::debug!(
                            tool_name = %tool_name,
                            tool_use_id = ?tool_use_id,
                            mode = "plan",
                            elapsed_us = elapsed.as_micros(),
                            "Tool allowed in Plan mode (read operation)"
                        );
                        return HookJsonOutput::Sync(SyncHookJsonOutput {
                            continue_: Some(true),
                            hook_specific_output: Some(HookSpecificOutput::PreToolUse(
                                PreToolUseHookSpecificOutput {
                                    permission_decision: Some("allow".to_string()),
                                    permission_decision_reason: Some(
                                        "Allowed in Plan mode (read operation)".to_string(),
                                    ),
                                    updated_input: None,
                                },
                            )),
                            ..Default::default()
                        });
                    }

                    // Check permission (if checker is available, otherwise default to Ask)
                    let permission_check = if let Some(checker) = &permission_checker {
                        let checker = checker.read().await;
                        checker.check_permission(&tool_name, &tool_input)
                    } else {
                        // No permission checker - default to Ask
                        crate::settings::PermissionCheckResult {
                            decision: crate::settings::PermissionDecision::Ask,
                            rule: None,
                            source: None,
                        }
                    };
                    let elapsed = start_time.elapsed();

                    // Record permission decision to span (batched for performance)
                    let span = tracing::Span::current();
                    span.record(
                        "permission_decision",
                        format!("{:?}", permission_check.decision),
                    );
                    span.record("check_duration_us", elapsed.as_micros());
                    if let Some(ref rule) = permission_check.rule {
                        span.record("permission_rule", rule.as_str());
                    }

                    tracing::info!(
                        tool_name = %tool_name,
                        tool_use_id = ?tool_use_id,
                        decision = ?permission_check.decision,
                        rule = ?permission_check.rule,
                        elapsed_us = elapsed.as_micros(),
                        "Permission check completed"
                    );

                    // 根据权限决策返回相应的 Hook 输出
                    // SDK 已修改为在 mcp_message 处理中调用 can_use_tool 回调，
                    // 因此 Ask 决策会由 SDK 层处理，Hook 只需要返回 continue_: true
                    match permission_check.decision {
                        crate::settings::PermissionDecision::Allow => {
                            tracing::debug!(
                                tool_name = %tool_name,
                                rule = ?permission_check.rule,
                                "Tool execution allowed by rule"
                            );
                            HookJsonOutput::Sync(SyncHookJsonOutput {
                                continue_: Some(true),
                                hook_specific_output: Some(HookSpecificOutput::PreToolUse(
                                    PreToolUseHookSpecificOutput {
                                        permission_decision: Some("allow".to_string()),
                                        permission_decision_reason: permission_check.rule,
                                        updated_input: None,
                                    },
                                )),
                                ..Default::default()
                            })
                        }
                        crate::settings::PermissionDecision::Deny => {
                            tracing::info!(
                                tool_name = %tool_name,
                                rule = ?permission_check.rule,
                                "Tool execution denied by rule"
                            );
                            HookJsonOutput::Sync(SyncHookJsonOutput {
                                continue_: Some(false), // 阻止执行
                                hook_specific_output: Some(HookSpecificOutput::PreToolUse(
                                    PreToolUseHookSpecificOutput {
                                        permission_decision: Some("deny".to_string()),
                                        permission_decision_reason: permission_check.rule,
                                        updated_input: None,
                                    },
                                )),
                                ..Default::default()
                            })
                        }
                        crate::settings::PermissionDecision::Ask => {
                            // Following TypeScript version's design:
                            // For "ask" decisions, we just return { continue: true } to let the
                            // normal permission flow continue. The actual permission request
                            // will be sent by the can_use_tool callback, NOT here.
                            //
                            // This ensures proper message ordering:
                            // 1. SDK processes tool_use -> sends session/update ToolCall
                            // 2. SDK calls can_use_tool callback
                            // 3. can_use_tool sends requestPermission() and waits for user response
                            //
                            // If we sent requestPermission() here (before can_use_tool), there
                            // could be race conditions with session/update notifications.

                            // Cache tool_use_id for can_use_tool callback to use
                            // The CLI doesn't always pass tool_use_id in mcp_message requests,
                            // so we cache it here where we have it.
                            if let Some(ref tuid) = tool_use_id {
                                let key = crate::session::stable_cache_key(&tool_input);
                                tracing::debug!(
                                    tool_name = %tool_name,
                                    tool_use_id = %tuid,
                                    "Caching tool_use_id for can_use_tool callback"
                                );
                                tool_use_id_cache.insert(key, tuid.clone());
                            }

                            tracing::debug!(
                                tool_name = %tool_name,
                                "Ask decision - delegating to can_use_tool callback"
                            );
                            HookJsonOutput::Sync(SyncHookJsonOutput {
                                continue_: Some(true),
                                hook_specific_output: None,
                                ..Default::default()
                            })
                        }
                    }
                }
                .instrument(span),
            ) as BoxFuture<'static, HookJsonOutput>
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{PermissionSettings, Settings};
    use serde_json::json;

    fn make_permission_checker(permissions: PermissionSettings) -> Arc<RwLock<PermissionChecker>> {
        let settings = Settings {
            permissions: Some(permissions),
            ..Default::default()
        };
        Arc::new(RwLock::new(PermissionChecker::new(settings, "/tmp")))
    }

    fn make_test_hook(checker: Arc<RwLock<PermissionChecker>>) -> HookCallback {
        make_test_hook_with_mode(checker, PermissionMode::Default)
    }

    fn make_test_hook_with_mode(
        checker: Arc<RwLock<PermissionChecker>>,
        mode: PermissionMode,
    ) -> HookCallback {
        let connection_cx_lock: Arc<OnceLock<JrConnectionCx<AgentToClient>>> =
            Arc::new(OnceLock::new());
        let permission_cache: Arc<DashMap<String, bool>> = Arc::new(DashMap::new());
        let tool_use_id_cache: Arc<DashMap<String, String>> = Arc::new(DashMap::new());
        create_pre_tool_use_hook(
            connection_cx_lock,
            "test-session".to_string(),
            Some(checker),
            Arc::new(RwLock::new(mode)),
            permission_cache,
            tool_use_id_cache,
        )
    }

    #[tokio::test]
    async fn test_pre_tool_use_hook_allow() {
        let checker = make_permission_checker(PermissionSettings {
            allow: Some(vec!["Read".to_string()]),
            ..Default::default()
        });

        let hook = make_test_hook(checker);
        let input = HookInput::PreToolUse(claude_code_agent_sdk::PreToolUseHookInput {
            session_id: "test".to_string(),
            transcript_path: "/tmp/test".to_string(),
            cwd: "/tmp".to_string(),
            permission_mode: None,
            tool_name: "Read".to_string(),
            tool_input: json!({"file_path": "/tmp/test.txt"}),
        });

        let result = hook(input, None, HookContext::default()).await;

        match result {
            HookJsonOutput::Sync(output) => {
                assert_eq!(output.continue_, Some(true));
                if let Some(HookSpecificOutput::PreToolUse(specific)) = output.hook_specific_output
                {
                    assert_eq!(specific.permission_decision, Some("allow".to_string()));
                } else {
                    panic!("Expected PreToolUse specific output");
                }
            }
            HookJsonOutput::Async(_) => panic!("Expected sync output"),
        }
    }

    // TODO: Re-enable when implementing permission checks
    // #[tokio::test]
    // async fn test_pre_tool_use_hook_deny() {
    //     let checker = make_permission_checker(PermissionSettings {
    //         deny: Some(vec!["Bash".to_string()]),
    //         ..Default::default()
    //     });
    //
    //     let hook = make_test_hook(checker);
    //     let input = HookInput::PreToolUse(claude_code_agent_sdk::PreToolUseHookInput {
    //         session_id: "test".to_string(),
    //         transcript_path: "/tmp/test".to_string(),
    //         cwd: "/tmp".to_string(),
    //         permission_mode: None,
    //         tool_name: "Bash".to_string(),
    //         tool_input: json!({"command": "ls"}),
    //     });
    //
    //     let result = hook(input, None, HookContext::default()).await;
    //
    //     match result {
    //         HookJsonOutput::Sync(output) => {
    //             assert_eq!(output.continue_, Some(true));
    //             if let Some(HookSpecificOutput::PreToolUse(specific)) = output.hook_specific_output
    //             {
    //                 assert_eq!(specific.permission_decision, Some("deny".to_string()));
    //             } else {
    //                 panic!("Expected PreToolUse specific output");
    //             }
    //         }
    //         HookJsonOutput::Async(_) => panic!("Expected sync output"),
    //     }
    // }

    #[tokio::test]
    async fn test_pre_tool_use_hook_ask_by_default() {
        // When no rules match, decision is "Ask".
        // Following TypeScript version's design, the hook just returns { continue: true }
        // to let the can_use_tool callback handle the permission request.
        let checker = make_permission_checker(PermissionSettings::default());
        let hook = make_test_hook(checker);

        // Test MCP tool - no matching rules means "Ask" decision,
        // hook returns continue=true with no hook_specific_output
        let input_mcp = HookInput::PreToolUse(claude_code_agent_sdk::PreToolUseHookInput {
            session_id: "test".to_string(),
            transcript_path: "/tmp/test".to_string(),
            cwd: "/tmp".to_string(),
            permission_mode: None,
            tool_name: "mcp__acp__Write".to_string(),
            tool_input: json!({"file_path": "/tmp/test.txt", "content": "test"}),
        });

        let result_mcp = hook(input_mcp, None, HookContext::default()).await;

        match result_mcp {
            HookJsonOutput::Sync(output) => {
                // Ask decision returns continue=true with no hook_specific_output
                // The actual permission request is handled by can_use_tool callback
                assert_eq!(output.continue_, Some(true));
                assert!(
                    output.hook_specific_output.is_none(),
                    "Ask decision should not set hook_specific_output"
                );
            }
            HookJsonOutput::Async(_) => panic!("Expected sync output"),
        }

        // Test built-in tool - same behavior
        let input_builtin = HookInput::PreToolUse(claude_code_agent_sdk::PreToolUseHookInput {
            session_id: "test".to_string(),
            transcript_path: "/tmp/test".to_string(),
            cwd: "/tmp".to_string(),
            permission_mode: None,
            tool_name: "Write".to_string(),
            tool_input: json!({"file_path": "/tmp/test.txt", "content": "test"}),
        });

        let result_builtin = hook(input_builtin, None, HookContext::default()).await;

        match result_builtin {
            HookJsonOutput::Sync(output) => {
                // Ask decision returns continue=true with no hook_specific_output
                assert_eq!(output.continue_, Some(true));
                assert!(
                    output.hook_specific_output.is_none(),
                    "Ask decision should not set hook_specific_output"
                );
            }
            HookJsonOutput::Async(_) => panic!("Expected sync output"),
        }
    }

    #[tokio::test]
    async fn test_pre_tool_use_hook_ignores_other_events() {
        let checker = make_permission_checker(PermissionSettings::default());

        let hook = make_test_hook(checker);
        let input = HookInput::PostToolUse(claude_code_agent_sdk::PostToolUseHookInput {
            session_id: "test".to_string(),
            transcript_path: "/tmp/test".to_string(),
            cwd: "/tmp".to_string(),
            permission_mode: None,
            tool_name: "Read".to_string(),
            tool_input: json!({}),
            tool_response: json!("content"),
        });

        let result = hook(input, None, HookContext::default()).await;

        match result {
            HookJsonOutput::Sync(output) => {
                assert_eq!(output.continue_, Some(true));
                assert!(output.hook_specific_output.is_none());
            }
            HookJsonOutput::Async(_) => panic!("Expected sync output"),
        }
    }

    #[tokio::test]
    async fn test_bypass_permissions_mode_allows_everything() {
        // BypassPermissions mode should allow all tools without checking rules
        let checker = make_permission_checker(PermissionSettings {
            deny: Some(vec!["Bash".to_string()]),
            ..Default::default()
        });

        let hook = make_test_hook_with_mode(checker, PermissionMode::BypassPermissions);
        let input = HookInput::PreToolUse(claude_code_agent_sdk::PreToolUseHookInput {
            session_id: "test".to_string(),
            transcript_path: "/tmp/test".to_string(),
            cwd: "/tmp".to_string(),
            permission_mode: None,
            tool_name: "Bash".to_string(),
            tool_input: json!({"command": "rm -rf /"}),
        });

        let result = hook(input, None, HookContext::default()).await;

        match result {
            HookJsonOutput::Sync(output) => {
                assert_eq!(output.continue_, Some(true));
                if let Some(HookSpecificOutput::PreToolUse(specific)) = output.hook_specific_output
                {
                    assert_eq!(specific.permission_decision, Some("allow".to_string()));
                    assert!(
                        specific
                            .permission_decision_reason
                            .unwrap()
                            .contains("BypassPermissions")
                    );
                } else {
                    panic!("Expected PreToolUse specific output");
                }
            }
            HookJsonOutput::Async(_) => panic!("Expected sync output"),
        }
    }

    #[tokio::test]
    async fn test_plan_mode_blocks_write_operations() {
        // Plan mode should block write operations (Edit, Write, Bash, NotebookEdit)
        let checker = make_permission_checker(PermissionSettings {
            allow: Some(vec!["Edit".to_string()]),
            ..Default::default()
        });

        let hook = make_test_hook_with_mode(checker, PermissionMode::Plan);
        let input = HookInput::PreToolUse(claude_code_agent_sdk::PreToolUseHookInput {
            session_id: "test".to_string(),
            transcript_path: "/tmp/test".to_string(),
            cwd: "/tmp".to_string(),
            permission_mode: None,
            tool_name: "Edit".to_string(),
            tool_input: json!({"file_path": "/tmp/test.txt"}),
        });

        let result = hook(input, None, HookContext::default()).await;

        match result {
            HookJsonOutput::Sync(output) => {
                assert_eq!(output.continue_, Some(true));
                if let Some(HookSpecificOutput::PreToolUse(specific)) = output.hook_specific_output
                {
                    assert_eq!(specific.permission_decision, Some("deny".to_string()));
                    assert!(
                        specific
                            .permission_decision_reason
                            .unwrap()
                            .contains("Plan mode")
                    );
                } else {
                    panic!("Expected PreToolUse specific output");
                }
            }
            HookJsonOutput::Async(_) => panic!("Expected sync output"),
        }
    }

    #[tokio::test]
    async fn test_plan_mode_allows_read_operations() {
        // Plan mode should allow read operations (without settings check)
        let checker = make_permission_checker(PermissionSettings::default());

        let hook = make_test_hook_with_mode(checker, PermissionMode::Plan);
        let input = HookInput::PreToolUse(claude_code_agent_sdk::PreToolUseHookInput {
            session_id: "test".to_string(),
            transcript_path: "/tmp/test".to_string(),
            cwd: "/tmp".to_string(),
            permission_mode: None,
            tool_name: "Read".to_string(),
            tool_input: json!({"file_path": "/tmp/test.txt"}),
        });

        let result = hook(input, None, HookContext::default()).await;

        match result {
            HookJsonOutput::Sync(output) => {
                assert_eq!(output.continue_, Some(true));
                // Plan mode allows reads - should return allow directly
                if let Some(HookSpecificOutput::PreToolUse(specific)) = output.hook_specific_output
                {
                    assert_eq!(specific.permission_decision, Some("allow".to_string()));
                } else {
                    panic!("Expected PreToolUse specific output");
                }
            }
            HookJsonOutput::Async(_) => panic!("Expected sync output"),
        }
    }

    #[tokio::test]
    async fn test_default_mode_respects_settings_rules() {
        // Default mode should respect settings rules
        let checker = make_permission_checker(PermissionSettings {
            allow: Some(vec!["Read".to_string()]),
            ..Default::default()
        });

        let hook = make_test_hook_with_mode(checker, PermissionMode::Default);
        let input = HookInput::PreToolUse(claude_code_agent_sdk::PreToolUseHookInput {
            session_id: "test".to_string(),
            transcript_path: "/tmp/test".to_string(),
            cwd: "/tmp".to_string(),
            permission_mode: None,
            tool_name: "Read".to_string(),
            tool_input: json!({"file_path": "/tmp/test.txt"}),
        });

        let result = hook(input, None, HookContext::default()).await;

        match result {
            HookJsonOutput::Sync(output) => {
                assert_eq!(output.continue_, Some(true));
                if let Some(HookSpecificOutput::PreToolUse(specific)) = output.hook_specific_output
                {
                    assert_eq!(specific.permission_decision, Some("allow".to_string()));
                } else {
                    panic!("Expected PreToolUse specific output");
                }
            }
            HookJsonOutput::Async(_) => panic!("Expected sync output"),
        }
    }
}
