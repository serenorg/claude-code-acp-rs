# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Claude Code ACP Agent implemented in Rust. Enables editors like Zed to use Claude Code capabilities via the ACP (Agent Client Protocol).

## Build & Development Commands

```bash
# Build and run (release)
cargo build --release && cargo run --release

# Run tests
cargo test --all-features --workspace

# Run a single test
cargo test <test_name> --all-features

# Lint with Clippy
cargo clippy --all-targets --all-features --workspace

# Format check
cargo fmt --all -- --check

# Format code
cargo fmt --all
```

## Architecture

### Core Modules

- **`agent/`** - ACP protocol handler and session management
  - `core.rs` - `ClaudeAcpAgent` main struct
  - `handlers.rs` - ACP request handlers (initialize, session/new, session/prompt)
  - `runner.rs` - Entry point (`run_acp()`)

- **`mcp/`** - Tool system (MCP pattern)
  - `acp_server.rs` - ACP-integrated MCP server with notification support
  - `registry.rs` - Tool registration and discovery
  - `tools/` - 22 built-in tools (bash, read, write, edit, grep, glob, etc.)
  - `external.rs` - External MCP server connections (framework for extensions)

- **`session/`** - Session lifecycle management
  - `manager.rs` - Concurrent session storage (uses DashMap)
  - `permission_manager.rs` - Tool permission decisions
  - `background_processes.rs` - Child process management (avoids zombies)
  - `usage.rs` - Token usage tracking

- **`settings/`** - Configuration system
  - `manager.rs` - Settings loading from `~/.claude/settings.json`, `.claude/settings.json`, `.claude/settings.local.json`
  - `permission_checker.rs` - Permission rule evaluation
  - `rule.rs` - Permission rule parsing (regex/glob patterns)
  - `watcher.rs` - File change watching for dynamic reload

- **`converter/`** - Protocol translation
  - `prompt.rs` - ACP requests → Claude SDK format
  - `notification.rs` - Claude SDK events → ACP notifications

- **`hooks/`** - Tool execution lifecycle
  - `pre_tool_use.rs` / `post_tool_use.rs` - Before/after tool execution callbacks

### Data Flow

```
ACP Client (Zed) → stdio → ClaudeAcpAgent → SessionManager → AcpMcpServer → Tools
                                ↓
                         Claude Agent SDK → Anthropic API
```

### Key Design Patterns

- **DashMap** for concurrent session storage (atomic operations, prefer entry API to avoid deadlocks)
- **Tool trait** (`mcp/tools/base.rs`) - All tools implement this trait
- **Permission system** - Three modes: allow/deny/ask with regex/glob pattern matching

## Configuration

Priority: Environment Variables > Settings top-level fields > Settings `env` object > Defaults

Key environment variables:
- `ANTHROPIC_API_KEY` - API key
- `ANTHROPIC_MODEL` - Model name (default: claude-sonnet-4-20250514)
- `MAX_THINKING_TOKENS` - Extended thinking mode budget

## Code Style Guidelines

- Use DashMap with entry API for concurrent Map operations to avoid deadlocks
- Follow SOLID principles
- Apply "Fail Fast" principle - surface errors early
- Avoid unsafe code unless required for FFI

## Commit Convention

Format: `<type>(<scope>): <summary>`

Types: `feat|fix|build|ci|docs|perf|refactor|test|chore`
