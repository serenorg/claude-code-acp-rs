# Introduction

## Project Alpha

I want to implement an ACP (Agent Client Protocol) agent in Rust using the Claude Code SDK for Rust:

- https://github.com/soddygo/claude-code-agent-sdk.git

Reference projects:

- https://github.com/zed-industries/claude-code-acp.git
  Zed's TypeScript implementation of a Claude Code ACP agent, built on the TypeScript Claude Code SDK.
  Local source: `vendors/claude-code-acp`
  Feature checklist: `specs/claude-code-acp/claude-code-acp-feature.md` (upstream may change and this doc may lag)

- https://github.com/anthropics/claude-agent-sdk-python.git
  Anthropic's Python SDK; used as a reference for the Rust SDK.
  Local source: `vendors/claude-agent-sdk-python`

- https://github.com/agentclientprotocol/rust-sdk.git
  The official Rust SDK for ACP (by Zed); we need to use this library.
  Local source: `vendors/acp-rust-sdk`
  SDK feature checklist: `specs/claude-code-acp/claude-agent-sdk-feature.md` (upstream may change and this doc may lag)

- https://github.com/agentclientprotocol/agent-client-protocol.git
  ACP schema definitions and SDKs across languages; all implementations are based on this spec.
  Local source: `vendors/agent-client-protocol`

- https://github.com/soddygo/claude-code-agent-sdk.git
  The Claude Code SDK source (mirrored in this repo under `vendors/claude-code-agent-sdk`).

After reviewing these projects, the goal is to implement a Rust ACP agent modeled after Zed's official TypeScript implementation.

Design principles:

- Use Rust edition 2024
- Use `tokio` as the async runtime / concurrency context
- The repo is a workspace (multiple crates under `crates/`), but the published crates.io package should remain a single crate for now

Requirements:

- Allow configuring the model via process env vars: `ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN`, `ANTHROPIC_MODEL`, `ANTHROPIC_SMALL_FAST_MODEL` (optional). This enables alternative endpoints (e.g. domestic proxies) for Claude Code.
- Track token usage for the current task run (if supported by the Anthropic SDK)
- Use `rmcp` for MCP integration (latest version)
- Manage dependency versions from the root `Cargo.toml`
- ACP APIs `new_session` and `load_session` include a `meta` field for extra info. We must support:
  - system prompt injection via `systemPrompt` (append/replace)
  - resuming a conversation via `session_id`

System prompt example:

```rust
let mut system_prompt_obj = serde_json::Map::new();
system_prompt_obj.insert(
    "append".to_string(),
    serde_json::Value::String(system_prompt.clone()),
);
meta.insert(
    "systemPrompt".to_string(),
    serde_json::Value::Object(system_prompt_obj),
);
```

Resume session_id example:

```rust
// Add session_id to resume, used for session recovery.
// Reference in the agent TypeScript code:
// resume: (params._meta as NewSessionMeta | undefined)?.claudeCode?.options?.resume
if let Some(ref session_id) = self.resume_session_id {
    // Build the claudeCode.options.resume structure
    let mut options = serde_json::Map::new();
    options.insert(
        "resume".to_string(),
        serde_json::Value::String(session_id.clone()),
    );

    let mut claude_code = serde_json::Map::new();
    claude_code.insert("options".to_string(), serde_json::Value::Object(options));

    meta.insert(
        "claudeCode".to_string(),
        serde_json::Value::Object(claude_code),
    );
}
```
