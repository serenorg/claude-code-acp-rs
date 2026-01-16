# Message Ordering Issue / 消息时序问题

## 问题描述

Agent 消息还没结束，UI 就显示结束了。然后在新任务中，之前的消息突然一下子收到了。

### 现象

1. Claude Agent 正在处理任务，发送多个 `session/update` 通知
2. 任务完成，返回 `EndTurn` 响应
3. 客户端（Zed）收到 `EndTurn`，认为任务结束
4. 之前的 `session/update` 通知还在队列中，尚未送达
5. 用户发起新任务后，之前的通知才到达，导致消息错乱

## 根本原因

`send_notification()` 使用 `unbounded_send()` 将消息放入异步队列，立即返回。消息的实际传输由后台 actor 异步处理。

```rust
// sacp/src/jsonrpc.rs
pub fn send_notification_to<Peer: JrPeer, N: JrNotification>(
    &self,
    peer: Peer,
    notification: N,
) -> Result<(), crate::Error> {
    // ...
    send_raw_message(
        &self.message_tx,  // unbounded channel
        OutgoingMessage::Notification { ... },
    )
}
```

当 `handle_prompt` 返回 `EndTurn` 时，队列中的通知可能还没有被完全发送给客户端。

## 当前的 Workaround

在 `src/agent/handlers.rs` 第 517-530 行，有一个基于通知数量的等待机制：

```rust
// 当前公式
let wait_ms = (10 + notification_count.saturating_mul(2)).min(100);
tokio::time::sleep(tokio::time::Duration::from_millis(wait_ms)).await;
```

**问题**：最大只等待 100ms，对于大量通知可能不够。

## 解决方案

### 方案 1：增加等待时间（临时方案）

```rust
// 改进公式：增加最大值，使用更激进的系数
let wait_ms = (20 + notification_count.saturating_mul(5)).min(300);
```

**优点**：简单，立即可用
**缺点**：不精确，可能等待过长或不够

### 方案 2：在 sacp 层添加 flush 机制（推荐）

在 `JrConnectionCx` 中添加 `flush()` 方法，等待所有待发送消息处理完成：

```rust
// 在 sacp/src/jsonrpc.rs 中添加

// 1. 添加新的消息类型
enum OutgoingMessage {
    // ... existing variants ...
    Flush { responder: oneshot::Sender<()> },
}

// 2. 在 JrConnectionCx 中添加方法
impl<Link: JrLink> JrConnectionCx<Link> {
    /// Wait for all pending outgoing messages to be sent
    pub async fn flush(&self) -> Result<(), crate::Error> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        send_raw_message(
            &self.message_tx,
            OutgoingMessage::Flush { responder: tx },
        )?;
        rx.await.map_err(|_| crate::Error::TransportClosed)
    }
}

// 3. 在 outgoing actor 中处理 Flush
// 当收到 Flush 消息时，说明之前的消息都已处理，回复 responder
```

**使用方式**：

```rust
// src/agent/handlers.rs
// 在返回 EndTurn 之前
connection_cx.flush().await?;
```

**优点**：精确，不会等待过长
**缺点**：需要修改 sacp 库

### 方案 3：使用消息序号（客户端配合）

在每个通知中添加序号，最后发送一个带有总数的 "sync" 通知，客户端等待收到所有消息后再认为任务结束。

```rust
// 每个通知带序号
session/update { seq: 1, ... }
session/update { seq: 2, ... }
session/update { seq: 3, ... }
// 最后发送同步通知
session/sync { total: 3 }
```

**优点**：可靠，不依赖时间
**缺点**：需要修改 ACP 协议和客户端

### 方案 4：tokio yield + 短延迟（折中方案）

```rust
// 给消息处理更多机会
for _ in 0..10 {
    tokio::task::yield_now().await;
}
tokio::time::sleep(Duration::from_millis(50)).await;
```

**优点**：比纯延迟更有效
**缺点**：仍然不精确

## 相关文件

- `src/agent/handlers.rs:517-530` - 当前的等待逻辑
- `vendors/symposium-acp/src/sacp/src/jsonrpc.rs:1818-1859` - send_notification 实现
- `vendors/symposium-acp/src/sacp/src/jsonrpc.rs:1761` - unbounded_send 调用

## 优先级

**中等** - 影响用户体验，但有 workaround

## 建议实施顺序

1. 先用方案 1 验证问题是否能通过增加等待时间解决
2. 如果方案 1 有效，考虑实施方案 2 作为长期解决方案
3. 方案 2 需要修改 sacp 库，可以提 PR 或 fork
