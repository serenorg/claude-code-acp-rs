# 权限系统实现说明

## 概述

本文档说明当前权限系统的状态、已解决的问题以及未来可能的改进。

## 当前状态

### ✅ 已实现

**权限规则基础框架**
- `src/settings/permission_checker.rs` - 权限规则检查器
- `src/permissions/can_use_tool.rs` - SDK `can_use_tool` 回调实现
- `src/session/permission_manager.rs` - 异步权限管理器框架
- `src/hooks/pre_tool_use.rs` - PreToolUse Hook 框架

**权限模式支持**
- `BypassPermissions` - 完全跳过权限检查
- `AcceptEdits` - 自动批准所有工具（与 BypassPermissions 行为相同，用于 root 兼容）
- `Plan` - 只读模式，阻止写操作
- `Default` - 正常权限检查模式
- `DontAsk` - 只允许预授权的工具

**设置文件支持**
- `~/.claude/settings.json` 中的权限规则配置
- 支持工具级别的 allow/deny 规则
- 支持通配符模式匹配（如 `Bash:npm *`）

**SDK 层 MCP 工具权限检查** (2026-01-15 实现)
- SDK 的 `mcp_message` 处理中调用 `can_use_tool` 回调
- 解决了原有架构中 MCP 工具不经过权限检查的问题
- 无死锁设计：回调在 spawned task 中执行

### 权限检查架构

```
┌─────────────────────────────────────────────────────────────────┐
│  权限检查流程 (2026-01-15 实现)                                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  mcp_message 请求                                               │
│       ↓                                                         │
│  SDK: extract_mcp_tool_info() 提取工具名和参数                    │
│       ↓                                                         │
│  SDK: can_use_tool() 回调 [spawned task，不阻塞事件循环]          │
│       ↓                                                         │
│  PermissionHandler.check_permission()                           │
│       │                                                         │
│       ├─ BypassPermissions/AcceptEdits → 直接允许               │
│       ├─ Plan 模式 → 阻止写操作 (Edit/Write/Bash/NotebookEdit)  │
│       ├─ Allow (规则匹配) → 返回 Allow                          │
│       ├─ Deny (规则匹配) → 返回 Deny，SDK 返回 JSON-RPC 错误     │
│       └─ Ask (无匹配规则) → 发送权限请求到客户端                  │
│                              ↓                                  │
│                          用户选择                               │
│                              ├─ Allow Once → 返回 Allow         │
│                              ├─ Always Allow → 添加规则 + Allow │
│                              └─ Reject → 返回 Deny              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 关键实现文件

| 文件 | 功能 |
|------|------|
| `vendors/claude-code-agent-sdk/src/internal/query_full.rs` | SDK 层 MCP 工具权限检查 |
| `src/hooks/pre_tool_use.rs` | PreToolUse Hook，处理 Allow/Deny/Ask 分支 |
| `src/permissions/can_use_tool.rs` | `can_use_tool` 回调实现 |
| `src/session/permission.rs` | PermissionHandler 和 PermissionMode |
| `src/settings/permission_checker.rs` | 权限规则检查器 |

## 已解决的核心问题

### 问题 1: SDK 架构限制 ✅ 已解决

**原问题：** SDK 不为 MCP 工具调用 `can_use_tool` 回调。

**解决方案：** 修改 SDK 的 `mcp_message` 处理，在执行 MCP 工具前调用 `can_use_tool` 回调。

```rust
// vendors/claude-code-agent-sdk/src/internal/query_full.rs
"mcp_message" => {
    // 提取工具信息
    if let Some((tool_name, tool_input)) = Self::extract_mcp_tool_info(mcp_message) {
        // 调用权限检查回调
        let result = callback(tool_name.clone(), tool_input, context).await;
        match result {
            PermissionResult::Deny(deny) => {
                // 返回 MCP 错误响应
                return Ok(json!({"mcp_response": {"error": {...}}}));
            }
            PermissionResult::Allow(_) => {
                // 继续执行
            }
        }
    }
    // 执行 MCP 工具...
}
```

### 问题 2: Hook 阻塞导致死锁 ✅ 已解决

**原问题：** 在 Hook 中等待权限响应会阻塞 SDK 事件循环导致死锁。

**解决方案：**
1. Hook 立即返回，不等待权限响应
2. SDK 的 `can_use_tool` 回调在 spawned task 中执行
3. 使用 oneshot channel 进行权限请求/响应通信

```rust
// src/hooks/pre_tool_use.rs
match permission_check.decision {
    PermissionDecision::Allow => {
        // 立即返回允许
    }
    PermissionDecision::Deny => {
        // 立即返回拒绝
    }
    PermissionDecision::Ask => {
        // 立即返回，让 SDK 的 can_use_tool 处理
        HookJsonOutput::Sync(SyncHookJsonOutput {
            continue_: Some(true),
            permission_decision: Some("ask".to_string()),
            ...
        })
    }
}
```

## 权限模式说明

| 模式 | 行为 | 用途 |
|------|------|------|
| `BypassPermissions` | 所有工具自动批准 | 开发/测试环境 |
| `AcceptEdits` | 所有工具自动批准（与 BypassPermissions 相同） | root 用户兼容 |
| `Plan` | 只允许读操作，阻止写操作 | 只读分析 |
| `Default` | 根据规则检查，无规则则询问用户 | 正常使用 |
| `DontAsk` | 只允许预授权的工具，其他拒绝 | 严格模式 |

## 验证测试

### 测试场景

| 场景 | 预期行为 | 状态 |
|------|----------|------|
| Default 模式，无规则 | 显示权限对话框 | ✅ |
| Default 模式，Allow Once | 工具执行一次 | ✅ |
| Default 模式，Always Allow | 工具执行，规则保存 | ✅ |
| Default 模式，已有 allow 规则 | 工具直接执行 | ✅ |
| Default 模式，有 deny 规则 | 工具被拒绝 | ✅ |
| Plan 模式 | 写操作被阻止 | ✅ |
| BypassPermissions 模式 | 所有工具直接执行 | ✅ |

### 测试命令

```bash
# 运行权限相关测试
cargo test permission --all-features

# 运行所有测试
cargo test --all-features

# Clippy 检查
cargo clippy --all-targets --all-features --workspace
```

## 用户交互工具

以下工具在所有模式下**自动批准**，因为它们本身是用户交互的机制：
- `AskUserQuestion` - AI 向用户提问
- `Task` - 子任务执行
- `TodoWrite` - 待办事项管理
- `SlashCommand` - 斜杠命令执行

## 配置示例

### 允许特定工具

```json
{
  "permissions": {
    "allow": ["Read", "Glob", "Grep", "Bash:npm *", "Bash:cargo *"]
  }
}
```

### 拒绝特定工具

```json
{
  "permissions": {
    "deny": ["Bash:rm *", "Bash:sudo *"]
  }
}
```

### 完整配置示例

```json
{
  "permissions": {
    "allow": [
      "Read",
      "Glob",
      "Grep",
      "Bash:npm *",
      "Bash:cargo *",
      "Bash:git *"
    ],
    "deny": [
      "Bash:rm -rf *",
      "Bash:sudo *"
    ]
  }
}
```

## 开发说明

### SDK 依赖配置

开发阶段使用本地 SDK：

```toml
# Cargo.toml
# 开发阶段：使用本地 SDK 路径依赖
claude-code-agent-sdk = { path = "vendors/claude-code-agent-sdk" }

# 发布阶段：恢复为 crates.io 版本
# claude-code-agent-sdk = { version = "0.1.22" }
```

### 为什么不会死锁

SDK 的控制请求处理在 spawned task 中执行：

```rust
// query_full.rs
tokio::spawn(async move {
    Self::handle_control_request_with_stdin(...).await
});
```

所以 `can_use_tool` 回调可以安全地执行异步操作（如发送权限请求到客户端并等待响应），不会阻塞 SDK 事件循环。

## 未来改进

### 可能的增强功能

1. **规则持久化改进** - 将 "Always Allow" 规则保存到配置文件
2. **更细粒度的规则** - 支持基于文件路径的规则（如只允许编辑特定目录）
3. **审计日志** - 记录所有权限决策用于安全审计
4. **超时机制** - 权限请求超时自动拒绝

### SDK flush 机制

向 symposium-acp 项目建议添加 `flush()` 机制，确保消息顺序：

```rust
// 建议的 API
pub async fn flush(&self) -> Result<(), Error> {
    let (done_tx, done_rx) = oneshot::channel();
    self.send_raw_message(OutgoingMessage::FlushMarker { done_tx })?;
    done_rx.await.map_err(|_| internal_error("Flush failed"))
}
```

## 参考实现

### Python SDK - anyio.TaskGroup 模式

```python
# query.py
async def _read_messages(self):
    async for message in self.transport.read_messages():
        if msg_type == "control_request":
            # 后台任务处理，主循环永不阻塞
            if self._tg:
                self._tg.start_soon(self._handle_control_request, request)
```

### Zed 编辑器 - 异步权限模式

```rust
// thread.rs
pub fn authorize(&self, title: impl Into<String>, cx: &mut App) -> Task<Result<()>> {
    let (response_tx, response_rx) = oneshot::channel();

    // 发送权限请求（不阻塞）
    self.stream.0.unbounded_send(Ok(ThreadEvent::ToolCallAuthorization(...)))?;

    // 后台任务等待响应
    cx.background_spawn(async move {
        match response_rx.await {
            Ok(option_id) => { /* 处理用户选择 */ }
            Err(Canceled) => { /* 处理取消 */ }
        }
    }).detach();
}
```

## 版本历史

| 版本 | 日期 | 说明 |
|------|------|------|
| 0.1.9 | 2026-01-15 | **实现完整权限系统** - SDK 层 MCP 工具权限检查，无死锁架构 |
| 0.1.7 | 2026-01-13 | 修复 Zed 客户端消息缓冲问题 - 实现动态延迟方案 |
| 0.1.4 | 2025-01-13 | 临时禁用权限检查 |
| 0.1.4 | 2025-01-13 | 默认权限模式改为 BypassPermissions |

---

## 已修复问题：Zed 客户端消息缓冲 (2026-01-13)

### 问题描述

使用 Zed 作为客户端时，agent 在发送 `EndTurn` 响应后，有时会出现消息缓冲现象。

### 解决方案

使用**动态延迟**方案：

```rust
// 基于通知数量计算等待时间
let wait_ms = (10 + notification_count.saturating_mul(2)).min(100);
tokio::time::sleep(tokio::time::Duration::from_millis(wait_ms)).await;
```

## 其他已知问题

| 问题 | 状态 | 优先级 |
|------|------|--------|
| Zed 消息缓冲 | ✅ 已修复 | 高 |
| Edit UTF-8 panic | ✅ 已修复 | 高 |
| 权限系统完整实现 | ✅ 已实现 | 高 |
| SDK flush 机制 | ⏳ 需要社区支持 | 低 |

---

**文档更新时间**: 2026-01-15
**维护者**: claude-code-acp-rs 团队
