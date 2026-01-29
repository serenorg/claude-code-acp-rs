# Claude Agent SDK for Rust Feature List

Based on analysis of the `vendors/claude-agent-sdk-rs` project, this document lists the features supported by the SDK.

**SDK info**:
- Version: 0.5.0
- Edition: 2024
- Minimum Rust version: 1.90
- Goal: 100% feature parity with the Python SDK

---

## 1. Project layout

```
vendors/claude-agent-sdk-rs/src/
├── lib.rs                # main library entry, public API exports
├── client.rs             # ClaudeClient bidirectional streaming client
├── query.rs              # simple query helpers
├── errors.rs             # error types
├── version.rs            # version + compatibility checks
├── types/                # public types
│   ├── config.rs         # configuration options
│   ├── messages.rs       # message types
│   ├── hooks.rs          # hook system
│   ├── permissions.rs    # permission management
│   ├── mcp.rs            # MCP protocol support
│   └── plugin.rs         # plugin configuration
└── internal/             # internal implementation
    ├── client.rs         # internal client
    ├── query_full.rs     # full query + bidirectional control
    ├── message_parser.rs # message parsing
    └── transport/        # transport layer
        ├── trait_def.rs  # Transport trait
        └── subprocess.rs # subprocess transport
```

---

## 2. Core client

### 2.1 ClaudeClient

| Feature | Method | Description | Source |
|--------|--------|-------------|--------|
| Create client | `ClaudeClient::new()` | Create a new client instance | client.rs |
| Create with validation | `ClaudeClient::try_new()` | Create and perform early validation | client.rs |
| Connect | `connect()` | Connect to the Claude CLI | client.rs |
| Disconnect | `disconnect()` | Disconnect from the CLI | client.rs |
| Simple query | `query()` | Send a text query | client.rs |
| Query with session | `query_with_session()` | Query with a specified session ID | client.rs |
| Query with content | `query_with_content()` | Send multimodal content | client.rs |
| Content + session query | `query_with_content_and_session()` | Multimodal content + session | client.rs |
| New session | `new_session()` | Create a new session | client.rs |
| Receive message stream | `receive_messages()` | Continuously receive all messages | client.rs |
| Receive response | `receive_response()` | Receive until a ResultMessage | client.rs |
| Interrupt | `interrupt()` | Interrupt the current operation | client.rs |
| Set permission mode | `set_permission_mode()` | Change permission mode dynamically | client.rs |
| Set model | `set_model()` | Switch models dynamically | client.rs |
| Rewind files | `rewind_files()` | Rewind files to a specific message state | client.rs |
| Get server info | `get_server_info()` | Get server init information | client.rs |

### 2.2 Simple query API

| Feature | Function | Return type | Source |
|--------|----------|------------|--------|
| Basic query | `query()` | `Vec<Message>` | query.rs |
| Streaming query | `query_stream()` | `Stream<Result<Message>>` | query.rs |
| Content query | `query_with_content()` | `Vec<Message>` | query.rs |
| Streaming content query | `query_stream_with_content()` | `Stream<Result<Message>>` | query.rs |

---

## 3. Message types

### 3.1 Top-level message enum

| Type | Description | Source |
|------|-------------|--------|
| `Message::Assistant` | Claude response | types/messages.rs |
| `Message::User` | User message | types/messages.rs |
| `Message::System` | System message | types/messages.rs |
| `Message::Result` | Query completion result | types/messages.rs |
| `Message::StreamEvent` | Streaming events | types/messages.rs |

### 3.2 Content block types

| Type | Description | Fields | Source |
|------|-------------|--------|--------|
| `ContentBlock::Text` | Text block | `text: String` | types/messages.rs |
| `ContentBlock::Thinking` | Thinking block (extended thinking) | `thinking: String, signature: String` | types/messages.rs |
| `ContentBlock::ToolUse` | Tool invocation block | `id, name, input` | types/messages.rs |
| `ContentBlock::ToolResult` | Tool result block | `tool_use_id, content, is_error` | types/messages.rs |
| `ContentBlock::Image` | Image block | `source: ImageSource` | types/messages.rs |

### 3.3 User content block types

| Type | Description | Source |
|------|-------------|--------|
| `UserContentBlock::Text` | User text | types/messages.rs |
| `UserContentBlock::Image` | User image | types/messages.rs |

### 3.4 Message struct details

| Struct | Key fields | Source |
|--------|------------|--------|
| `AssistantMessage` | message, parent_tool_use_id, session_id, uuid | types/messages.rs |
| `UserMessage` | text, content, uuid, parent_tool_use_id | types/messages.rs |
| `SystemMessage` | subtype, cwd, session_id, tools, mcp_servers, model | types/messages.rs |
| `ResultMessage` | duration_ms, is_error, num_turns, session_id, total_cost_usd, usage | types/messages.rs |
| `StreamEvent` | uuid, session_id, event, parent_tool_use_id | types/messages.rs |

---

## 4. Hook system (Hooks)

### 4.1 Hook event types

| Event | Trigger | Input type | Source |
|------|---------|------------|--------|
| `PreToolUse` | Before a tool is used | `PreToolUseHookInput` | types/hooks.rs |
| `PostToolUse` | After a tool is used | `PostToolUseHookInput` | types/hooks.rs |
| `UserPromptSubmit` | When the user submits a prompt | `UserPromptSubmitHookInput` | types/hooks.rs |
| `Stop` | When execution stops | `StopHookInput` | types/hooks.rs |
| `SubagentStop` | When a sub-agent stops | `SubagentStopHookInput` | types/hooks.rs |
| `PreCompact` | Before conversation compaction | `PreCompactHookInput` | types/hooks.rs |

### 4.2 Hook input structs

| Struct | Key fields | Source |
|--------|------------|--------|
| `PreToolUseHookInput` | session_id, transcript_path, cwd, permission_mode, tool_name, tool_input | types/hooks.rs |
| `PostToolUseHookInput` | same as above + tool_response | types/hooks.rs |
| `UserPromptSubmitHookInput` | session_id, transcript_path, cwd, permission_mode, prompt | types/hooks.rs |

### 4.3 Hook output types

| Type | Description | Source |
|------|-------------|--------|
| `HookJsonOutput::Sync` | Synchronous output (blocks execution) | types/hooks.rs |
| `HookJsonOutput::Async` | Asynchronous output (runs in background) | types/hooks.rs |

### 4.4 Sync hook output fields

| Field | Type | Description | Source |
|------|------|-------------|--------|
| `continue_` | `Option<bool>` | Whether to continue execution | types/hooks.rs |
| `suppress_output` | `Option<bool>` | Whether to suppress output | types/hooks.rs |
| `stop_reason` | `Option<String>` | Stop reason | types/hooks.rs |
| `decision` | `Option<String>` | Permission decision | types/hooks.rs |
| `system_message` | `Option<String>` | System message | types/hooks.rs |
| `reason` | `Option<String>` | Decision reason | types/hooks.rs |

### 4.5 Hooks Builder API

| Method | Description | Source |
|--------|-------------|--------|
| `Hooks::new()` | Create a new Hooks builder | types/hooks.rs |
| `add_pre_tool_use()` | Add a global PreToolUse hook | types/hooks.rs |
| `add_pre_tool_use_with_matcher()` | Add a hook for a specific tool | types/hooks.rs |
| `add_post_tool_use()` | Add a PostToolUse hook | types/hooks.rs |
| `add_post_tool_use_with_matcher()` | Add post-processing for a specific tool | types/hooks.rs |
| `add_user_prompt_submit()` | Add a user prompt hook | types/hooks.rs |
| `add_stop()` | Add a stop hook | types/hooks.rs |
| `add_subagent_stop()` | Add a sub-agent stop hook | types/hooks.rs |
| `add_pre_compact()` | Add a pre-compaction hook | types/hooks.rs |
| `build()` | Build the final configuration | types/hooks.rs |

---

## 5. Permission system

### 5.1 Permission modes

| Mode | Description | Source |
|------|-------------|--------|
| `PermissionMode::Default` | Prompt the user for confirmation | types/permissions.rs |
| `PermissionMode::AcceptEdits` | Automatically accept edits | types/permissions.rs |
| `PermissionMode::Plan` | Planning mode | types/permissions.rs |
| `PermissionMode::BypassPermissions` | Bypass all permission checks | types/permissions.rs |

### 5.2 Permission callback

| Type | Description | Source |
|------|-------------|--------|
| `CanUseToolCallback` | Callback for tool permission checks | types/permissions.rs |

### 5.3 Permission result types

| Type | Description | Source |
|------|-------------|--------|
| `PermissionResult::Allow` | Allow execution (optionally with modified input) | types/permissions.rs |
| `PermissionResult::Deny` | Deny execution (with message + interrupt flag) | types/permissions.rs |

### 5.4 Permission update types

| Type | Description | Source |
|------|-------------|--------|
| `AddRules` | Add permission rules | types/permissions.rs |
| `ReplaceRules` | Replace permission rules | types/permissions.rs |
| `RemoveRules` | Remove permission rules | types/permissions.rs |
| `SetMode` | Set permission mode | types/permissions.rs |
| `AddDirectories` | Add directories | types/permissions.rs |
| `RemoveDirectories` | Remove directories | types/permissions.rs |

### 5.5 Permission behaviors

| Behavior | Description | Source |
|----------|-------------|--------|
| `PermissionBehavior::Allow` | Allow | types/permissions.rs |
| `PermissionBehavior::Deny` | Deny | types/permissions.rs |
| `PermissionBehavior::Ask` | Ask the user | types/permissions.rs |

---

## 6. MCP server support

### 6.1 Server types

| Type | Config struct | Description | Source |
|------|---------------|-------------|--------|
| Stdio | `McpStdioServerConfig` | Standard IO transport | types/mcp.rs |
| SSE | `McpSseServerConfig` | Server-Sent Events transport | types/mcp.rs |
| HTTP | `McpHttpServerConfig` | HTTP transport | types/mcp.rs |
| Sdk | `McpSdkServerConfig` | In-process MCP server | types/mcp.rs |

### 6.2 Server configuration

| Config | Fields | Source |
|--------|--------|--------|
| `McpStdioServerConfig` | command, args, env | types/mcp.rs |
| `McpSseServerConfig` | url, headers | types/mcp.rs |
| `McpHttpServerConfig` | url, headers | types/mcp.rs |
| `McpSdkServerConfig` | name, instance | types/mcp.rs |

### 6.3 SDK MCP tools

| Type | Description | Source |
|------|-------------|--------|
| `SdkMcpTool` | SDK MCP tool definition | types/mcp.rs |
| `ToolHandler` | Tool handler trait | types/mcp.rs |
| `ToolResult` | Tool execution result | types/mcp.rs |
| `ToolResultContent` | Tool result content (Text/Image) | types/mcp.rs |

### 6.4 Helper functions

| Function | Description | Source |
|----------|-------------|--------|
| `create_sdk_mcp_server()` | Create an SDK MCP server | types/mcp.rs |

---

## 7. Configuration options (ClaudeAgentOptions)

### 7.1 Basic configuration

| Option | Type | Description | Source |
|--------|------|-------------|--------|
| `model` | `Option<String>` | Model selection | types/config.rs |
| `fallback_model` | `Option<String>` | Fallback model | types/config.rs |
| `max_turns` | `Option<u32>` | Maximum turns | types/config.rs |
| `max_budget_usd` | `Option<f64>` | USD budget limit | types/config.rs |
| `max_thinking_tokens` | `Option<u32>` | Extended thinking token limit | types/config.rs |

### 7.2 Tool configuration

| Option | Type | Description | Source |
|--------|------|-------------|--------|
| `tools` | `Option<Tools>` | Tools configuration (list or preset) | types/config.rs |
| `allowed_tools` | `Vec<String>` | Allowed tool list | types/config.rs |
| `disallowed_tools` | `Vec<String>` | Disallowed tool list | types/config.rs |
| `mcp_servers` | `McpServers` | MCP server configuration | types/config.rs |

### 7.3 System configuration

| Option | Type | Description | Source |
|--------|------|-------------|--------|
| `system_prompt` | `Option<SystemPrompt>` | System prompt (text or preset) | types/config.rs |
| `permission_mode` | `Option<PermissionMode>` | Permission mode | types/config.rs |
| `permission_prompt_tool_name` | `Option<String>` | Tool name used for permission prompts | types/config.rs |

### 7.4 Session management

| Option | Type | Description | Source |
|--------|------|-------------|--------|
| `resume` | `Option<String>` | Session ID to resume | types/config.rs |
| `fork_session` | `bool` | Whether to fork a new session each time | types/config.rs |
| `continue_conversation` | `bool` | Whether to continue an existing conversation | types/config.rs |

### 7.5 Environment configuration

| Option | Type | Description | Source |
|--------|------|-------------|--------|
| `cwd` | `Option<PathBuf>` | Working directory | types/config.rs |
| `cli_path` | `Option<PathBuf>` | CLI path | types/config.rs |
| `settings` | `Option<String>` | Settings file | types/config.rs |
| `add_dirs` | `Vec<PathBuf>` | Additional directories | types/config.rs |
| `env` | `HashMap<String, String>` | Environment variables | types/config.rs |

### 7.6 Advanced features

| Option | Type | Description | Source |
|--------|------|-------------|--------|
| `hooks` | `Option<HashMap<HookEvent, Vec<HookMatcher>>>` | Hook configuration | types/config.rs |
| `can_use_tool` | `Option<CanUseToolCallback>` | Permission check callback | types/config.rs |
| `plugins` | `Vec<SdkPluginConfig>` | Plugin configuration | types/config.rs |
| `sandbox` | `Option<SandboxSettings>` | Sandbox settings | types/config.rs |
| `enable_file_checkpointing` | `bool` | File checkpointing | types/config.rs |

### 7.7 Streaming

| Option | Type | Description | Source |
|--------|------|-------------|--------|
| `include_partial_messages` | `bool` | Include partial messages | types/config.rs |

### 7.8 Output configuration

| Option | Type | Description | Source |
|--------|------|-------------|--------|
| `output_format` | `Option<serde_json::Value>` | Output format (JSON Schema) | types/config.rs |

### 7.9 Other options

| Option | Type | Description | Source |
|--------|------|-------------|--------|
| `user` | `Option<String>` | User identifier | types/config.rs |
| `setting_sources` | `Option<Vec<SettingSource>>` | Settings sources | types/config.rs |
| `agents` | `Option<HashMap<String, AgentDefinition>>` | Custom agents | types/config.rs |
| `betas` | `Vec<SdkBeta>` | Beta features | types/config.rs |
| `extra_args` | `HashMap<String, Option<String>>` | Extra CLI args | types/config.rs |
| `max_buffer_size` | `Option<usize>` | Buffer size | types/config.rs |
| `stderr_callback` | `Option<Arc<dyn Fn(String) + Send + Sync>>` | stderr callback | types/config.rs |

### 7.10 System prompt configuration

| Type | Description | Source |
|------|-------------|--------|
| `SystemPrompt::Text(String)` | Direct text prompt | types/config.rs |
| `SystemPrompt::Preset(SystemPromptPreset)` | Preset (includes append) | types/config.rs |

### 7.11 Tools configuration

| Type | Description | Source |
|------|-------------|--------|
| `Tools::List(Vec<String>)` | List of tool names | types/config.rs |
| `Tools::Preset(ToolsPreset)` | Preset | types/config.rs |

### 7.12 Settings sources

| Source | Description | Source file |
|--------|-------------|------------|
| `SettingSource::User` | ~/.claude/settings.json | types/config.rs |
| `SettingSource::Project` | .claude/settings.json | types/config.rs |
| `SettingSource::Local` | .claude/settings.local.json (highest priority) | types/config.rs |

### 7.13 Agent definition

| Field | Type | Description | Source |
|------|------|-------------|--------|
| `description` | `String` | Agent description | types/config.rs |
| `prompt` | `String` | Agent prompt | types/config.rs |
| `tools` | `Option<Vec<String>>` | Available tools | types/config.rs |
| `model` | `Option<AgentModel>` | Agent model | types/config.rs |

### 7.14 Agent models

| Model | Description | Source |
|------|-------------|--------|
| `AgentModel::Sonnet` | Claude Sonnet | types/config.rs |
| `AgentModel::Opus` | Claude Opus | types/config.rs |
| `AgentModel::Haiku` | Claude Haiku | types/config.rs |
| `AgentModel::Inherit` | Inherit parent model | types/config.rs |

### 7.15 Sandbox settings

| Field | Type | Description | Source |
|------|------|-------------|--------|
| `enabled` | `Option<bool>` | Enable sandbox | types/config.rs |
| `auto_allow_bash_if_sandboxed` | `Option<bool>` | Auto-allow bash when sandboxed | types/config.rs |
| `excluded_commands` | `Option<Vec<String>>` | Excluded commands | types/config.rs |
| `allow_unsandboxed_commands` | `Option<bool>` | Allow unsandboxed commands | types/config.rs |
| `network` | `Option<SandboxNetworkConfig>` | Network configuration | types/config.rs |

### 7.16 Builder API

| Method | Description | Source |
|--------|-------------|--------|
| `ClaudeAgentOptions::builder()` | Create a builder | types/config.rs |
| `.model()` | Set model | types/config.rs |
| `.fallback_model()` | Set fallback model | types/config.rs |
| `.max_budget_usd()` | Set budget | types/config.rs |
| `.max_thinking_tokens()` | Set thinking token limit | types/config.rs |
| `.max_turns()` | Set max turns | types/config.rs |
| `.permission_mode()` | Set permission mode | types/config.rs |
| `.plugins()` | Set plugins | types/config.rs |
| `.build()` | Build config | types/config.rs |

---

## 8. Error handling

### 8.1 Main error type

| Error variant | Description | Source |
|--------------|-------------|--------|
| `ClaudeError::Connection` | CLI connection error | errors.rs |
| `ClaudeError::Process` | Process error | errors.rs |
| `ClaudeError::JsonDecode` | JSON decode error | errors.rs |
| `ClaudeError::MessageParse` | Message parse error | errors.rs |
| `ClaudeError::Transport` | Transport error | errors.rs |
| `ClaudeError::ControlProtocol` | Control protocol error | errors.rs |
| `ClaudeError::InvalidConfig` | Invalid configuration | errors.rs |
| `ClaudeError::CliNotFound` | CLI not found | errors.rs |
| `ClaudeError::ImageValidation` | Image validation error | errors.rs |
| `ClaudeError::Io` | IO error | errors.rs |
| `ClaudeError::Other` | Other error | errors.rs |

### 8.2 Specific error structs

| Error struct | Fields | Source |
|-------------|--------|--------|
| `ConnectionError` | message | errors.rs |
| `ProcessError` | message, exit_code, stderr | errors.rs |
| `JsonDecodeError` | message, line | errors.rs |
| `MessageParseError` | message, data | errors.rs |
| `CliNotFoundError` | message, cli_path | errors.rs |
| `ImageValidationError` | message | errors.rs |

### 8.3 Image validation constraints

| Constraint | Value | Source |
|-----------|-------|--------|
| Supported MIME types | image/jpeg, image/png, image/gif, image/webp | errors.rs |
| Base64 max size | 15MB (about 20MB after decode) | errors.rs |

---

## 9. Version management

| Feature | Description | Source |
|--------|-------------|--------|
| `SDK_VERSION` | Current SDK version | version.rs |
| `MIN_CLI_VERSION` | Minimum required CLI version (2.0.0) | version.rs |
| `SKIP_VERSION_CHECK_ENV` | Env var to skip version checks | version.rs |
| `parse_version()` | Parse a version string | version.rs |
| `check_version()` | Check CLI version compatibility | version.rs |

---

## 10. Notable features

### 10.1 Multimodal input support

| Feature | Method | Description | Source |
|--------|--------|-------------|--------|
| Base64 image | `UserContentBlock::image_base64()` | Create an image from base64 | types/messages.rs |
| URL image | `UserContentBlock::image_url()` | Create an image from a URL | types/messages.rs |

### 10.2 Extended thinking support

| Feature | Description | Source |
|--------|-------------|--------|
| `ThinkingBlock` | Capture model reasoning output | types/messages.rs |
| `max_thinking_tokens` | Limit thinking token count | types/config.rs |

### 10.3 Cost control

| Feature | Description | Source |
|--------|-------------|--------|
| `max_budget_usd` | USD budget limit | types/config.rs |
| `fallback_model` | Fallback model when the primary fails | types/config.rs |
| `ResultMessage.total_cost_usd` | Query cost | types/messages.rs |

### 10.4 File checkpointing

| Feature | Description | Source |
|--------|-------------|--------|
| `enable_file_checkpointing` | Enable file change tracking | types/config.rs |
| `rewind_files()` | Rewind files to a specific user message state | client.rs |

### 10.5 Plugin system

| Feature | Description | Source |
|--------|-------------|--------|
| `SdkPluginConfig::local()` | Load a local plugin | types/plugin.rs |

---

## Feature counts

| Category | Count |
|----------|-------|
| Client methods | 16 |
| Simple query APIs | 4 |
| Message types | 5 |
| Content block types | 5 |
| User content block types | 2 |
| Hook events | 6 |
| Hook builder methods | 10 |
| Permission modes | 4 |
| Permission update types | 6 |
| MCP server types | 4 |
| Config options | 30+ |
| Error variants | 11 |
| **Total** | **100+** |

---

## Dependencies

```
Main dependencies:
- tokio (1.48)         # async runtime
- async-trait (0.1)    # async traits
- futures (0.3)        # Future utilities
- serde (1.0)          # serialization
- serde_json (1.0)     # JSON
- thiserror (2.0)      # error types
- anyhow (1.0)         # error handling
- tracing (0.1)        # logging
- uuid (1.19)          # UUID generation
- typed-builder        # builder pattern
```

---

## Changelog

- 2024-01-07: Initial version based on analysis of claude-agent-sdk-rs v0.5.0
