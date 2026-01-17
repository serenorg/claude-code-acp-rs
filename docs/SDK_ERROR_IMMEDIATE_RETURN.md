# SDK Immediate Error Return Issue / SDK 立即返回错误问题

## 问题描述

在某些情况下，当用户在 Zed 中发送消息后，Zed 的发送按钮立即变成可发送状态（表示任务已完成），但当用户稍等片刻后重新发送消息时，会突然看到一堆之前的 agent 消息涌出。

### 用户复现场景

1. 在 Zed 中发送消息
2. Zed 的发送按钮立即变成可发送状态（似乎任务已完成）
3. 稍等片刻后重新发送消息
4. 突然看到一堆 agent 消息出现

### 日志证据

```log
# 正常请求流程（请求A）
02:58:20.443 - Starting prompt processing
02:58:28.327 - Prompt completed (7884ms, 正常)

# 问题请求（请求B）
02:58:56.276 - Starting prompt processing
02:58:56.276 - Sending query to Claude CLI
02:58:56.276 - Received message from SDK (仅 0.064ms 后！)
02:58:56.276 - Received ResultMessage: error_during_execution
02:58:56.288 - Returning EndTurn (立即返回)

# 下一个请求（请求C）
02:59:07.026 - Starting prompt processing (11秒后)
03:01:xx - 正常处理中...
```

## 根本原因

### SDK 内部状态问题

SDK 在处理请求 B 时**立即**返回了 `error_during_execution`，而不是正常处理请求：

```
从发送 query 到收到 ResultMessage: 0.064ms
正常请求应该需要: 几秒到几分钟
```

这说明 SDK 遇到了内部错误，可能的原因：

1. **竞争条件**：上一个请求 A 刚完成，SDK 内部状态还没完全重置
2. **请求队列问题**：SDK 可能还在处理某个内部清理工作
3. **资源未释放**：某些资源（如进程、连接、句柄）还没有完全释放
4. **内部状态机错误**：SDK 的状态机可能处于不一致的状态

### 错误消息携带的错误信息

```log
Received ResultMessage from Claude CLI
subtype=error_during_execution
duration_ms=7882    # 这是上一个请求A的duration！
num_turns=4          # 这是上一个请求A的turns！
```

错误消息中携带的 `duration_ms` 和 `num_turns` 属于**上一个请求A**，而不是当前请求B，这进一步证实了 SDK 内部状态混乱。

## 时序分析

### 正常流程

```
用户发送消息
    ↓
Agent 发送 query 到 SDK
    ↓
SDK 处理（几秒到几分钟）
    ↓
SDK 返回 ResultMessage(success)
    ↓
Agent 返回 EndTurn
    ↓
Zed 收到 EndTurn，恢复按钮状态
```

### 异常流程

```
用户发送消息
    ↓
Agent 发送 query 到 SDK
    ↓
SDK 立即返回 ResultMessage(error_during_execution) ← 仅 0.064ms！
    ↓
Agent 返回 EndTurn
    ↓
Zed 收到 EndTurn，立即恢复按钮状态 ← 用户误以为任务完成
    ↓
用户重新发送消息
    ↓
SDK 此时已准备好，正常处理
    ↓
[可能] 之前请求B的部分结果突然到达 ← "一堆agent消息"
```

## 代码位置

### Agent 处理 error_during_execution 的代码

**文件**: `src/agent/handlers.rs:570-576`

```rust
let stop_reason = match result.subtype.as_str() {
    "success" | "error_during_execution" => {
        tracing::debug!(
            session_id = %session_id,
            subtype = %result.subtype,
            "Returning EndTurn for result subtype"
        );
        StopReason::EndTurn
    }
    // ...
};
```

**分析**：
- Agent 将 `error_during_execution` 视为正常情况处理
- 直接返回 `EndTurn`，没有特殊处理
- 这导致 Zed 立即恢复按钮状态

## 解决方案

### ✅ 已实施修复：返回 Refusal 而非 EndTurn

**问题根源**：对比 TypeScript 实现发现，当 CLI 返回 `error_during_execution` 时：

| 实现 | 返回值 | 结果 |
|------|--------|------|
| **TypeScript** | `stopReason: "refusal"` | Zed 知道是错误，不会立即让用户重试 |
| **Rust (修复前)** | `StopReason::EndTurn` | Zed 认为正常完成，立即让用户重试 |

**修复代码** (`src/agent/handlers.rs:580-590`):

```rust
"error_during_execution" => {
    // Match TS behavior: return Refusal for error_during_execution
    // This signals to the client that execution failed, preventing
    // immediate retry while CLI internal state recovers
    tracing::info!(
        session_id = %session_id,
        subtype = %result.subtype,
        "Returning Refusal for error_during_execution (CLI execution error)"
    );
    StopReason::Refusal  // 之前是 EndTurn
}
```

**同时修复了**：未知 subtype 也返回 `Refusal`（而非 `EndTurn`），匹配 TS 行为：

```rust
_ => {
    // Match TS behavior: unknown subtypes return Refusal (not EndTurn)
    tracing::warn!(
        session_id = %session_id,
        subtype = %result.subtype,
        "Unknown result subtype, returning Refusal"
    );
    StopReason::Refusal  // 之前是 EndTurn
}
```

**修复效果**：
- ✅ Zed 收到 `Refusal`，知道是错误状态
- ✅ 用户不会看到"立即恢复可用"的发送按钮
- ✅ 避免 CLI 内部状态未恢复时的立即重试
- ✅ 与 TypeScript 实现行为一致

### 临时 Workaround（未采用）

在收到 `error_during_execution` 后，添加短暂延迟再返回 EndTurn，给 SDK 更多时间清理：

```rust
// 备选方案（未采用）
tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
StopReason::EndTurn
```

**未采用原因**：返回 `Refusal` 是更正确的协议行为，不需要额外的延迟。

### 根本解决方案

**向 CLI 报告此 bug**，让 CLI：
1. 正确处理内部状态管理
2. 在准备好接收新请求之前返回正确的错误
3. 避免在状态转换期返回误导性的 `error_during_execution`

## 相关问题

### 与 Flush 机制的交互

本项目实现了 flush 机制（`docs/MESSAGE_ORDERING_ISSUE.md`）来解决消息时序问题。但 `error_during_execution` 是一个不同层面的问题：

| 问题 | 原因 | 解决方案 |
|------|------|----------|
| Message Ordering | SDK 异步发送通知，EndTurn 可能在通知之前到达 | Flush 机制 |
| Immediate Error Return | SDK 内部状态问题，立即返回错误 | SDK bug 修复 |

## 诊断命令

### 检查日志中的异常模式

```bash
# 查找 error_during_execution
grep "error_during_execution" /tmp/container-logs/*.log

# 查找快速完成的请求（<1秒）
grep "Prompt completed.*total_elapsed_ms=[0-9]" /tmp/container-logs/*.log

# 查找 EndTurn 后立即开始的新请求
grep -A 1 "Returning EndTurn" /tmp/container-logs/*.log | grep "Starting prompt"
```

### 分析请求时序

```bash
# 查看所有请求的开始和结束时间
grep -E "Starting prompt processing|Prompt completed" /tmp/container-logs/claude-code-acp-rs-*.log
```

## 优先级

**中等** - 影响 UX，但这是 SDK 层面的问题，agent 层面的 workaround 有限。

## TODO

- [ ] 向 SDK 团队报告此 bug
- [ ] 收集更多复现场景和数据
- [ ] 评估是否需要添加 workaround
- [ ] 监控生产环境中的错误率

## 相关文件

| 文件 | 说明 |
|------|------|
| `src/agent/handlers.rs:570-576` | 处理 ResultMessage 的代码 |
| `docs/MESSAGE_ORDERING_ISSUE.md` | 消息时序问题文档 |

## 参考资料

- [Rust Async Programming](https://rust-lang.github.io/async-book/)
- [State Machine Design Patterns](https://refactoring.guru/design-patterns/state)
