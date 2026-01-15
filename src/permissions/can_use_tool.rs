//! can_use_tool callback implementation for SDK
//!
//! This module implements the permission checking callback that the SDK calls
//! before executing tools. Following the TypeScript version's design, this callback
//! is responsible for sending permission requests when needed.

use claude_code_agent_sdk::types::permissions::{
    CanUseToolCallback, PermissionResult, PermissionResultAllow, PermissionResultDeny,
    ToolPermissionContext,
};
use std::sync::{Arc, OnceLock};
use tracing::{debug, info, warn};

use crate::session::{PermissionOutcome, PermissionRequestBuilder, Session, ToolPermissionResult};

/// Create a can_use_tool callback that receives Session via OnceLock
///
/// Following TypeScript version's design, this callback:
/// 1. Checks permission rules first
/// 2. For "ask" decisions, sends requestPermission() and waits for user response
/// 3. Returns the appropriate allow/deny result
pub fn create_can_use_tool_callback(
    session_lock: Arc<OnceLock<Arc<Session>>>,
) -> CanUseToolCallback {
    Arc::new(
        move |tool_name: String, tool_input: serde_json::Value, context: ToolPermissionContext| {
            let session_lock = Arc::clone(&session_lock);

            Box::pin(async move {
                debug!(
                    tool_name = %tool_name,
                    tool_use_id = ?context.tool_use_id,
                    "can_use_tool callback called"
                );

                // Try to get session from OnceLock
                let Some(session) = session_lock.get() else {
                    warn!(
                        tool_name = %tool_name,
                        "Session not ready in callback - denying"
                    );
                    return PermissionResult::Deny(PermissionResultDeny {
                        message: "Session not initialized yet".to_string(),
                        interrupt: false,
                    });
                };

                // Check permission handler first
                let handler_guard = session.permission().await;
                let result = handler_guard
                    .check_permission(&tool_name, &tool_input)
                    .await;
                drop(handler_guard); // Release the lock before async operations

                match result {
                    ToolPermissionResult::Allowed => {
                        info!(
                            tool_name = %tool_name,
                            "Permission allowed by handler"
                        );
                        PermissionResult::Allow(PermissionResultAllow::default())
                    }
                    ToolPermissionResult::Blocked { reason } => {
                        info!(
                            tool_name = %tool_name,
                            reason = %reason,
                            "Permission blocked by handler"
                        );
                        PermissionResult::Deny(PermissionResultDeny {
                            message: reason,
                            interrupt: false,
                        })
                    }
                    ToolPermissionResult::NeedsPermission => {
                        // This is the "ask" case - send permission request to client
                        // Following TypeScript version's design
                        info!(
                            tool_name = %tool_name,
                            "Permission needed - sending request to client"
                        );

                        // Get tool_use_id from context, or from cache if not provided
                        // The cache is populated by pre_tool_use hook when Ask decision is made
                        let tool_use_id = match context.tool_use_id {
                            Some(id) => id,
                            None => {
                                // Try to get from cache (populated by pre_tool_use hook)
                                match session.get_cached_tool_use_id(&tool_input) {
                                    Some(cached_id) => {
                                        debug!(
                                            tool_name = %tool_name,
                                            cached_tool_use_id = %cached_id,
                                            "Using cached tool_use_id from hook"
                                        );
                                        cached_id
                                    }
                                    None => {
                                        warn!(
                                            tool_name = %tool_name,
                                            "No tool_use_id in context or cache - denying for security"
                                        );
                                        return PermissionResult::Deny(PermissionResultDeny {
                                            message:
                                                "No tool_use_id available for permission request"
                                                    .to_string(),
                                            interrupt: false,
                                        });
                                    }
                                }
                            }
                        };

                        // Get connection_cx from session
                        let Some(connection_cx) = session.get_connection_cx() else {
                            warn!(
                                tool_name = %tool_name,
                                "Connection not ready - denying for security"
                            );
                            return PermissionResult::Deny(PermissionResultDeny {
                                message: "Connection not ready for permission request".to_string(),
                                interrupt: false,
                            });
                        };

                        // Send permission request and wait for response
                        let outcome = PermissionRequestBuilder::new(
                            &session.session_id,
                            &tool_use_id,
                            &tool_name,
                            tool_input.clone(),
                        )
                        .request(connection_cx)
                        .await;

                        match outcome {
                            Ok(PermissionOutcome::AllowOnce) => {
                                info!(tool_name = %tool_name, "Permission allowed once by user");
                                PermissionResult::Allow(PermissionResultAllow::default())
                            }
                            Ok(PermissionOutcome::AllowAlways) => {
                                info!(tool_name = %tool_name, "Permission allowed always by user");
                                // Add rule to permission checker for future invocations
                                let handler_guard = session.permission().await;
                                handler_guard.add_allow_rule_for_tool_call(&tool_name, &tool_input);
                                drop(handler_guard);
                                PermissionResult::Allow(PermissionResultAllow::default())
                            }
                            Ok(PermissionOutcome::Rejected) | Ok(PermissionOutcome::Cancelled) => {
                                info!(tool_name = %tool_name, "Permission rejected/cancelled by user");
                                PermissionResult::Deny(PermissionResultDeny {
                                    message: "User denied permission".to_string(),
                                    interrupt: false,
                                })
                            }
                            Err(e) => {
                                warn!(
                                    tool_name = %tool_name,
                                    error = %e,
                                    "Permission request failed"
                                );
                                PermissionResult::Deny(PermissionResultDeny {
                                    message: format!("Permission request failed: {}", e),
                                    interrupt: false,
                                })
                            }
                        }
                    }
                }
            })
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: The callback now requires Arc<OnceLock<Arc<Session>>>
    // which requires a full Session setup to test.
    // The basic test below verifies the callback compiles correctly.
    // Functional tests require integration testing with a real session.

    #[test]
    fn test_callback_function_compiles() {
        // This test verifies the callback function signature is correct
        let session_lock: Arc<OnceLock<Arc<Session>>> = Arc::new(OnceLock::new());
        let _callback = create_can_use_tool_callback(session_lock);
        // If this compiles, the signature is correct
    }
}
