# Claude Code ACP Agent Feature Checklist

Based on a feature analysis of Zed's official TypeScript implementation (`vendors/claude-code-acp`), this document lists the capabilities that should be implemented in the Rust version.

---

## 1. ACP protocol request support

### 1.1 Core lifecycle requests

| Feature | Request Type | Status | TS Source |
|--------|--------------|--------|----------|
| Initialize | `InitializeRequest` → `InitializeResponse` | [ ] | acp-agent.ts:157-203 |
| Authenticate | `AuthenticateRequest` | [ ] | acp-agent.ts:248-250 |
| New session | `NewSessionRequest` → `NewSessionResponse` | [ ] | acp-agent.ts:205-217 |
| Fork session | `ForkSessionRequest` → `ForkSessionResponse` | [ ] | acp-agent.ts:219-231 |
| Resume session | `ResumeSessionRequest` → `ResumeSessionResponse` | [ ] | acp-agent.ts:233-246 |

### 1.2 Session interaction requests

| Feature | Request Type | Status | TS Source |
|--------|--------------|--------|----------|
| Prompt | `PromptRequest` → `PromptResponse` | [ ] | acp-agent.ts:252-410 |
| Cancel | `CancelNotification` | [ ] | acp-agent.ts:412-418 |
| Set session model | `SetSessionModelRequest` → `SetSessionModelResponse` | [ ] | acp-agent.ts:420-427 |
| Set session mode | `SetSessionModeRequest` → `SetSessionModeResponse` | [ ] | acp-agent.ts:429-453 |

### 1.3 File operation requests

| Feature | Request Type | Status | TS Source |
|--------|--------------|--------|----------|
| Read text file | `ReadTextFileRequest` → `ReadTextFileResponse` | [ ] | acp-agent.ts:455-458 |
| Write text file | `WriteTextFileRequest` → `WriteTextFileResponse` | [ ] | acp-agent.ts:460-463 |

---

## 2. Session management

### 2.1 Session lifecycle

| Feature | Description | Status | TS Source |
|--------|-------------|--------|----------|
| Session creation | Create a new Claude Code session; return sessionId, available models, and modes | [ ] | acp-agent.ts:593-838 |
| Session fork | Create a branch from an existing session while preserving context | [ ] | acp-agent.ts:219-231 |
| Session resume | Restore a previous session state via the `resume` option | [ ] | acp-agent.ts:233-246, 215 |
| Session tracking | Maintain a session map `sessions: { [sessionId]: Session }` | [ ] | acp-agent.ts:141-143 |
| User input management | Use `Pushable<SDKUserMessage>` to handle the user message stream | [ ] | acp-agent.ts:608 |

### 2.2 Session configuration

| Setting | Description | Status | TS Source |
|--------|-------------|--------|----------|
| Working directory | Set the session working directory via `params.cwd` | [ ] | acp-agent.ts:679 |
| System prompt | Support built-in or custom system prompts (append/replace) | [ ] | acp-agent.ts:649-661 |
| MCP server merging | Merge user-provided MCP servers with ACP built-in MCP servers | [ ] | acp-agent.ts:615-647 |
| Permission mode | Default to "default" mode | [ ] | acp-agent.ts:663 |
| Settings sources | Support `{"user", "project", "local"}` | [ ] | acp-agent.ts:675 |
| Cancellation signal | Support `AbortController`-style cancellation | [ ] | acp-agent.ts:767-770 |

---

## 3. Permission system

### 3.1 Permission decision flow

| Feature | Description | Status | TS Source |
|--------|-------------|--------|----------|
| Permission entry point | `canUseTool()` callback | [ ] | acp-agent.ts:465-591 |
| ExitPlanMode special-case | Special permission logic when exiting Plan mode | [ ] | acp-agent.ts:476-525 |
| Auto-approve rules | Auto-approve in bypassPermissions and acceptEdits modes | [ ] | acp-agent.ts:527-538 |
| Permission prompt | Request permission from the client (dialog) | [ ] | acp-agent.ts:540-589 |
| Rule application | Apply predefined rules via SettingsManager | [ ] | tools.ts:652-696 |

### 3.2 Permission modes

| Mode | Description | Status | TS Source |
|------|-------------|--------|----------|
| default | Standard behavior; prompt on dangerous actions | [ ] | acp-agent.ts:801-804 |
| acceptEdits | Automatically accept file edits | [ ] | acp-agent.ts:806-809 |
| plan | Planning mode; do not execute real tools | [ ] | acp-agent.ts:811-814 |
| dontAsk | Do not prompt; reject anything not pre-approved | [ ] | acp-agent.ts:816-819 |
| bypassPermissions | Bypass all permission checks (non-root user) | [ ] | acp-agent.ts:822-828 |

### 3.3 Settings management

| Feature | Description | Status | TS Source |
|--------|-------------|--------|----------|
| Multi-layer loading | user → project → local → enterprise-managed settings | [ ] | settings.ts:244-252 |
| Permission rule formats | `"ToolName"`, `"ToolName(argument)"`, `"ToolName(prefix:*)"` | [ ] | settings.ts:9-18 |
| File watching | Watch all settings files for changes | [ ] | settings.ts:380-412 |
| Rule precedence | Deny > Allow > Ask | [ ] | settings.ts:451-473 |
| Glob matching | Use glob patterns for file paths | [ ] | settings.ts:137-146 |
| Shell operator guard | Prevent command injection via `&&`, `||`, `;`, `\|`, etc. | [ ] | settings.ts:62-64 |

---

## 4. Tool invocation support

### 4.1 ACP built-in tools (MCP server registration)

| Tool | Fully-qualified name | Description | Status | TS Source |
|------|----------------------|-------------|--------|----------|
| Read | mcp__acp__Read | Read file contents (supports line offset and limit) | [ ] | mcp-server.ts:105-209 |
| Write | mcp__acp__Write | Write a file (must read first) | [ ] | mcp-server.ts:212-270 |
| Edit | mcp__acp__Edit | Edit via exact string replacement | [ ] | mcp-server.ts:272-361 |
| Bash | mcp__acp__Bash | Execute bash commands (foreground/background) | [ ] | mcp-server.ts:365-512 |
| BashOutput | mcp__acp__BashOutput | Fetch background bash output | [ ] | mcp-server.ts:514-570 |
| KillShell | mcp__acp__KillShell | Kill a background bash process | [ ] | mcp-server.ts:572-636 |

### 4.2 Claude Code SDK native tools (conversion support)

| Tool | Description | Status | TS Source |
|------|-------------|--------|----------|
| Task | Task planning | [ ] | tools.ts:58-71 |
| NotebookRead | Read Jupyter notebooks | [ ] | tools.ts:73-79 |
| NotebookEdit | Edit Jupyter notebooks | [ ] | tools.ts:81-95 |
| Bash | Native bash (when built-ins are disabled) | [ ] | tools.ts:97-111 |
| BashOutput | Native output polling | [ ] | tools.ts:113-119 |
| KillShell | Native process kill | [ ] | tools.ts:121-127 |
| LS | Directory listing | [ ] | tools.ts:167-173 |
| Glob | File pattern search | [ ] | tools.ts:242-256 |
| Grep | Text search | [ ] | tools.ts:258-321 |
| WebFetch | Fetch a web page | [ ] | tools.ts:323-336 |
| WebSearch | Search the web | [ ] | tools.ts:338-354 |
| TodoWrite | TODO list management | [ ] | tools.ts:356-363 |
| ExitPlanMode | Exit planning mode | [ ] | tools.ts:365-373 |

### 4.3 Tool disable mechanism

| Feature | Description | Status | TS Source |
|--------|-------------|--------|----------|
| Disable built-in tools | Disable via `disableBuiltInTools` metadata | [ ] | acp-agent.ts:716 |
| Selective enabling | Enable tools based on client capabilities | [ ] | acp-agent.ts:719-729 |
| Blacklist management | Disable unnecessary tools | [ ] | acp-agent.ts:732-756 |

---

## 5. Notification types (Session Updates)

### 5.1 Message notifications

| Notification type | Description | Status | TS Source |
|------------------|-------------|--------|----------|
| agent_message_chunk | Agent text message chunks (streaming) | [ ] | acp-agent.ts:1001 |
| user_message_chunk | User text message chunks | [ ] | acp-agent.ts:1001 |
| agent_thought_chunk | Agent internal reasoning chunks | [ ] | acp-agent.ts:1040 |

### 5.2 Tool notifications

| Notification type | Description | Status | TS Source |
|------------------|-------------|--------|----------|
| tool_call | Tool call started (pending) | [ ] | acp-agent.ts:1100 |
| tool_call_update | Tool call update / completion | [ ] | acp-agent.ts:1073, 1133 |

### 5.3 Session management notifications

| Notification type | Description | Status | TS Source |
|------------------|-------------|--------|----------|
| current_mode_update | Permission mode change | [ ] | acp-agent.ts:506 |
| available_commands_update | Available slash commands update | [ ] | acp-agent.ts:793 |
| plan | Planning update | [ ] | acp-agent.ts:1055 |

### 5.4 Content types

| Content type | Description | Status | TS Source |
|-------------|-------------|--------|----------|
| text | Plain text content | [ ] | acp-agent.ts:1003 |
| image | Image data (base64 or URL) | [ ] | acp-agent.ts:1026-1035 |
| diff | File diff (tool result) | [ ] | tools.ts:186-190 |
| content | Generic content block | [ ] | tools.ts:65-70 |
| terminal | Terminal output | [ ] | mcp-server.ts:433 |

---

## 6. Meta field support

### 6.1 New session meta

| Field path | Type | Description | Status | TS Source |
|-----------|------|-------------|--------|----------|
| `_meta.claudeCode.options` | Options | Claude Code SDK options | [ ] | acp-agent.ts:96-112 |
| `_meta.claudeCode.options.resume` | string | Session ID to resume | [ ] | acp-agent.ts:215 |
| `_meta.disableBuiltInTools` | boolean | Disable ACP built-in tools | [ ] | acp-agent.ts:640 |
| `_meta.systemPrompt` | string \| {append: string} | Custom system prompt | [ ] | acp-agent.ts:650-661 |

### 6.2 Tool call meta

| Field path | Description | Status | TS Source |
|-----------|-------------|--------|----------|
| `_meta.claudeCode.toolName` | Native tool name executed | [ ] | acp-agent.ts:1096 |
| `_meta.claudeCode.toolResponse` | Structured tool response | [ ] | acp-agent.ts:1068 |

---

## 7. MCP Server functionality

### 7.1 MCP server integration

| Feature | Description | Status | TS Source |
|--------|-------------|--------|----------|
| Built-in ACP MCP server | Register an MCP server named "acp" | [ ] | acp-agent.ts:641-646 |
| Merge user MCP servers | Merge MCP server configuration from ACP requests | [ ] | acp-agent.ts:616-637 |
| stdio transport support | Support stdio MCP servers | [ ] | acp-agent.ts:618-628 |
| HTTP transport support | Support URL (HTTP) MCP servers | [ ] | acp-agent.ts:629-635 |
| Conditional tool registration | Register tools selectively based on client capabilities | [ ] | mcp-server.ts:104, 211, 364 |

### 7.2 MCP tool behaviors

| Feature | Description | Status | TS Source |
|--------|-------------|--------|----------|
| Internal path access | Allow access to internal `~/.claude` files for persistence | [ ] | mcp-server.ts:50-56 |
| Settings file protection | Block access to sensitive files like `settings.json` | [ ] | mcp-server.ts:53-54 |
| Line offset and limit | File reads support line offsets and line-count limits | [ ] | mcp-server.ts:63-78 |
| Byte limit | Enforce a 50KB read limit | [ ] | mcp-server.ts:29, 169 |

---

## 8. Special features

### 8.1 Background terminal management

| Feature | Description | Status | TS Source |
|--------|-------------|--------|----------|
| Run in background | Enable background execution via `run_in_background: true` | [ ] | mcp-server.ts:388-392 |
| Background terminal tracking | Track active sessions via a `backgroundTerminals` map | [ ] | acp-agent.ts:146 |
| Terminal handle | Return a terminal ID for subsequent querying and control | [ ] | mcp-server.ts:459-493 |
| Output polling | BashOutput tool fetches incremental output | [ ] | mcp-server.ts:533-569 |
| Process control | Support kill/timeout/abort termination modes | [ ] | mcp-server.ts:447-484 |

### 8.2 File editing

| Feature | Description | Status | TS Source |
|--------|-------------|--------|----------|
| Exact replacement | Edit tool supports exact string replacement | [ ] | mcp-server.ts:330-336 |
| Global replacement | `replace_all` supports whole-file replacement | [ ] | mcp-server.ts:294-298 |
| Diff generation | Use a diff library to produce patch output | [ ] | mcp-server.ts:338 |
| Uniqueness check | Ensure `old_string` is unique in the file | [ ] | mcp-server.ts:286 |

### 8.3 Prompt handling

| Feature | Description | Status | TS Source |
|--------|-------------|--------|----------|
| Multi-type support | Support text, resource links, resources, and images | [ ] | acp-agent.ts:905-981 |
| MCP command conversion | `/mcp:server:command` → `/server:command (MCP)` | [ ] | acp-agent.ts:914-918 |
| Context wrapping | Wrap resource text in `<context>` tags | [ ] | acp-agent.ts:937-940 |
| URI formatting | Format file:// and zed:// URIs into Markdown links | [ ] | acp-agent.ts:888-903 |

### 8.4 Hook system

| Hook | Description | Status | TS Source |
|------|-------------|--------|----------|
| PreToolUse | Permission check before tool use | [ ] | tools.ts:652-696 |
| PostToolUse | Capture structured responses after tool use | [ ] | tools.ts:631-645 |
| Hook callback registry | Dynamically register and clean up hook callbacks | [ ] | tools.ts:613-645 |

### 8.5 Slash commands

| Feature | Description | Status | TS Source |
|--------|-------------|--------|----------|
| Command discovery | `getAvailableSlashCommands()` returns all supported commands | [ ] | acp-agent.ts:860-886 |
| Unsupported command filtering | Filter out commands like context, cost, login, logout, etc. | [ ] | acp-agent.ts:861-869 |
| MCP command identification | Handle commands marked "(MCP)" | [ ] | acp-agent.ts:876-877 |

### 8.6 Model management

| Feature | Description | Status | TS Source |
|--------|-------------|--------|----------|
| Model discovery | `getAvailableModels()` returns supported models | [ ] | acp-agent.ts:841-858 |
| Model switching | `setModel()` switches the model in-session | [ ] | acp-agent.ts:426 |
| Initial model selection | Use the first available model as the initial selection | [ ] | acp-agent.ts:845-846 |

### 8.7 Streaming

| Feature | Description | Status | TS Source |
|--------|-------------|--------|----------|
| Content block streaming | Handle content_block_start/delta/stop events | [ ] | acp-agent.ts:1171-1194 |
| Text delta | Accumulate text incrementally from text_delta blocks | [ ] | acp-agent.ts:1017-1024 |
| Thinking delta | Capture Claude reasoning via thinking_delta | [ ] | acp-agent.ts:1037-1045 |
| NDJSON stream | Use ndJsonStream for protocol communication | [ ] | acp-agent.ts:1206 |

---

## Feature summary

| Category | Count | Implemented | Completion |
|---------|-------|-------------|------------|
| ACP protocol requests | 12 | 0 | 0% |
| Session management | 11 | 0 | 0% |
| Permission system | 17 | 0 | 0% |
| Tool invocation | 22 | 0 | 0% |
| Notification types | 11 | 0 | 0% |
| Meta fields | 6 | 0 | 0% |
| MCP Server | 9 | 0 | 0% |
| Special features | 26 | 0 | 0% |
| **Total** | **114** | **0** | **0%** |

---

## TypeScript project file structure reference

```
vendors/claude-code-acp/src/
├── index.ts                    # entry and exports
├── lib.ts                      # library exports
├── acp-agent.ts                # ACP agent implementation (core, ~1209 lines)
├── mcp-server.ts               # MCP server and tool registration (~807 lines)
├── tools.ts                    # tool info + conversion logic (~697 lines)
├── settings.ts                 # permission system + settings management (~523 lines)
├── utils.ts                    # stream conversion + utilities (~172 lines)
└── tests/
    ├── acp-agent.test.ts       # integration tests
    ├── settings.test.ts        # settings tests
    ├── extract-lines.test.ts   # line extraction tests
    └── replace-and-calculate-location.test.ts
```

---

## Dependency mapping reference

```
Core dependencies:
- @agentclientprotocol/sdk (v0.12.0)        # ACP protocol implementation
- @anthropic-ai/claude-agent-sdk (v0.1.73)  # Claude Code SDK
- @modelcontextprotocol/sdk (v1.25.1)       # MCP protocol implementation
- diff (v8.0.2)                             # file diff generation
- minimatch (v10.1.1)                       # glob pattern matching

Rust equivalents:
- sacp                         # ACP protocol Rust SDK
- claude-agent-sdk-rs          # Claude Code SDK for Rust
- rmcp                         # MCP protocol Rust SDK
- similar / diffy              # file diff generation
- glob / globset               # glob pattern matching
```

---

## Changelog

- 2024-01-07: Initial version based on analysis of the TS implementation
