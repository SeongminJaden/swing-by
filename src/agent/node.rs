//! 에이전트 노드 통신 시스템
#![allow(dead_code)]
//!
//! 여러 에이전트 노드가 채널을 통해 메시지를 주고받으며 협업.
//! 아키텍처:
//!   NodeHub — 모든 노드의 채널을 관리하는 중앙 허브
//!   AgentNode — 개별 에이전트 (송신/수신 채널 보유)
//!
//! 메시지 흐름:
//!   사용자 → Orchestrator Node → [Planner, Developer, Debugger] 노드들
//!   각 노드 → 결과를 다음 노드로 전달 → 최종 결과 수집

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

// ─── 노드 메시지 ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct NodeMessage {
    pub from: String,       // 발신 노드 이름
    pub to: String,         // 수신 노드 이름 (빈 문자열 = 브로드캐스트)
    pub msg_type: MsgType,
    pub content: String,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MsgType {
    Task,       // 작업 요청
    Result,     // 결과 반환
    Status,     // 상태 업데이트
    Error,      // 오류
    Control,    // 제어 명령 (pause, resume, cancel)
}

impl NodeMessage {
    pub fn task(from: &str, to: &str, content: &str) -> Self {
        Self {
            from: from.to_string(),
            to: to.to_string(),
            msg_type: MsgType::Task,
            content: content.to_string(),
            metadata: HashMap::new(),
        }
    }

    pub fn result(from: &str, to: &str, content: &str) -> Self {
        Self {
            from: from.to_string(),
            to: to.to_string(),
            msg_type: MsgType::Result,
            content: content.to_string(),
            metadata: HashMap::new(),
        }
    }

    pub fn status(from: &str, content: &str) -> Self {
        Self {
            from: from.to_string(),
            to: String::new(),
            msg_type: MsgType::Status,
            content: content.to_string(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_meta(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
}

// ─── 노드 허브 ───────────────────────────────────────────────────────────────

pub type NodeSender = async_channel::Sender<NodeMessage>;
pub type NodeReceiver = async_channel::Receiver<NodeMessage>;

#[derive(Clone)]
pub struct NodeHub {
    channels: Arc<Mutex<HashMap<String, NodeSender>>>,
    log: Arc<Mutex<Vec<NodeMessage>>>,  // 메시지 로그
}

impl NodeHub {
    pub fn new() -> Self {
        Self {
            channels: Arc::new(Mutex::new(HashMap::new())),
            log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 노드를 등록하고 (송신 채널, 수신 채널) 반환
    pub async fn register(&self, name: &str) -> (NodeSender, NodeReceiver) {
        let (tx, rx) = async_channel::bounded(64);
        self.channels.lock().await.insert(name.to_string(), tx.clone());
        (tx, rx)
    }

    /// 특정 노드에 메시지 전송
    pub async fn send(&self, msg: NodeMessage) -> Result<()> {
        let channels = self.channels.lock().await;
        if msg.to.is_empty() {
            // 브로드캐스트: 발신자 제외 모든 노드
            for (name, tx) in channels.iter() {
                if name != &msg.from {
                    let _ = tx.try_send(msg.clone());
                }
            }
        } else if let Some(tx) = channels.get(&msg.to) {
            tx.send(msg.clone()).await
                .map_err(|_| anyhow::anyhow!("노드 '{}' 채널 닫힘", msg.to))?;
        } else {
            anyhow::bail!("노드 '{}' 를 찾을 수 없음", msg.to);
        }

        // 로그 기록
        drop(channels);
        self.log.lock().await.push(msg);
        Ok(())
    }

    /// 등록된 노드 목록
    pub async fn node_names(&self) -> Vec<String> {
        self.channels.lock().await.keys().cloned().collect()
    }

    /// 메시지 로그 조회
    pub async fn message_log(&self) -> Vec<NodeMessage> {
        self.log.lock().await.clone()
    }

    /// 특정 노드 제거
    pub async fn unregister(&self, name: &str) {
        self.channels.lock().await.remove(name);
    }
}

// ─── 에이전트 노드 ───────────────────────────────────────────────────────────

pub struct AgentNode {
    pub name: String,
    hub: NodeHub,
    rx: NodeReceiver,
    #[allow(dead_code)]
    tx: NodeSender,
}

impl AgentNode {
    pub async fn new(name: &str, hub: &NodeHub) -> Self {
        let (tx, rx) = hub.register(name).await;
        Self {
            name: name.to_string(),
            hub: hub.clone(),
            rx,
            tx,
        }
    }

    /// 다른 노드에 메시지 전송
    pub async fn send(&self, to: &str, msg_type: MsgType, content: &str) -> Result<()> {
        let msg = NodeMessage {
            from: self.name.clone(),
            to: to.to_string(),
            msg_type,
            content: content.to_string(),
            metadata: HashMap::new(),
        };
        self.hub.send(msg).await
    }

    /// 메시지 수신 (비차단)
    pub fn try_recv(&self) -> Option<NodeMessage> {
        self.rx.try_recv().ok()
    }

    /// 메시지 수신 (차단, 타임아웃 있음)
    pub async fn recv_timeout(&self, timeout_ms: u64) -> Option<NodeMessage> {
        tokio::time::timeout(
            std::time::Duration::from_millis(timeout_ms),
            self.rx.recv()
        ).await.ok().and_then(|r| r.ok())
    }

    /// 상태 브로드캐스트
    pub async fn broadcast_status(&self, content: &str) -> Result<()> {
        self.hub.send(NodeMessage::status(&self.name, content)).await
    }
}

// ─── Pipeline 노드 실행 ─────────────────────────────────────────────────────

/// 허브를 통해 여러 에이전트 노드가 순차적으로 작업을 처리하는 Pipeline
/// Planner → Developer → Debugger 형태로 결과를 전달
pub async fn run_node_pipeline(
    hub: &NodeHub,
    client: &crate::agent::ollama::OllamaClient,
    task: &str,
    on_status: impl Fn(&str) + Clone + Send + 'static,
) -> Result<String> {
    use crate::agent::orchestrator::{run_agent, AgentRole};

    let roles = [
        ("planner", AgentRole::Planner),
        ("developer", AgentRole::Developer),
        ("debugger", AgentRole::Debugger),
    ];

    // 모든 노드 등록
    let mut nodes = Vec::new();
    for (name, _) in &roles {
        let node = AgentNode::new(name, hub).await;
        nodes.push(node);
    }

    let mut context = String::new();
    let mut final_result = String::new();

    for (i, ((name, role), node)) in roles.iter().zip(nodes.iter()).enumerate() {
        let status = format!("[{}] 시작", name.to_uppercase());
        let _ = node.broadcast_status(&status).await;
        on_status(&status);

        let output = run_agent(
            client,
            *role,
            task,
            &context,
            12,
            {
                let on_status = on_status.clone();
                let name = name.to_string();
                move |msg| on_status(&format!("[{}] {}", name.to_uppercase(), msg))
            },
        ).await;

        let result_str = output.content.clone();
        final_result = result_str.clone();

        // 다음 노드에 결과 전달
        if i + 1 < roles.len() {
            let next_name = roles[i + 1].0;
            let msg = NodeMessage::result(name, next_name, &result_str);
            let _ = hub.send(msg).await;
            context = format!("이전 단계 ({}) 결과:\n{}", name, crate::utils::trunc(&result_str, 1500));
        }

        let done_status = format!("[{}] 완료 (툴: {}회)", name.to_uppercase(), output.tool_calls_made);
        let _ = node.broadcast_status(&done_status).await;
        on_status(&done_status);
    }

    // 모든 노드 해제
    for (name, _) in &roles {
        hub.unregister(name).await;
    }

    Ok(final_result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn hub_register_and_send() {
        let hub = NodeHub::new();
        let (_tx, rx) = hub.register("alpha").await;

        hub.send(NodeMessage::task("beta", "alpha", "hello")).await.unwrap();

        let msg = rx.try_recv().expect("메시지가 있어야 함");
        assert_eq!(msg.from, "beta");
        assert_eq!(msg.content, "hello");
        assert_eq!(msg.msg_type, MsgType::Task);
    }

    #[tokio::test]
    async fn hub_broadcast_skips_sender() {
        let hub = NodeHub::new();
        let (_atx, arx) = hub.register("A").await;
        let (_btx, brx) = hub.register("B").await;
        let (_ctx, crx) = hub.register("C").await;

        // A가 브로드캐스트 → B, C에 전달되고 A 자신은 수신 안 함
        hub.send(NodeMessage::status("A", "broadcast")).await.unwrap();

        assert!(arx.try_recv().is_err(), "sender must not receive own broadcast");
        assert!(brx.try_recv().is_ok(), "B must receive");
        assert!(crx.try_recv().is_ok(), "C must receive");
    }

    #[tokio::test]
    async fn hub_send_to_unknown_node_errors() {
        let hub = NodeHub::new();
        let result = hub.send(NodeMessage::task("me", "nobody", "hi")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn hub_message_log_records_sent_msgs() {
        let hub = NodeHub::new();
        let (_tx, _rx) = hub.register("A").await;
        let (_tx2, _rx2) = hub.register("B").await;
        hub.send(NodeMessage::result("A", "B", "done")).await.unwrap();
        let log = hub.message_log().await;
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].content, "done");
    }

    #[tokio::test]
    async fn hub_unregister_removes_node() {
        let hub = NodeHub::new();
        hub.register("X").await;
        assert_eq!(hub.node_names().await.len(), 1);
        hub.unregister("X").await;
        assert_eq!(hub.node_names().await.len(), 0);
    }

    #[tokio::test]
    async fn agent_node_send_and_recv() {
        let hub = NodeHub::new();
        let a = AgentNode::new("A", &hub).await;
        let b = AgentNode::new("B", &hub).await;

        a.send("B", MsgType::Task, "work").await.unwrap();

        let msg = b.try_recv().expect("B should have a message");
        assert_eq!(msg.content, "work");
        assert_eq!(msg.from, "A");
    }

    #[tokio::test]
    async fn node_message_with_meta() {
        let msg = NodeMessage::task("src", "dst", "data")
            .with_meta("key", "value");
        assert_eq!(msg.metadata.get("key").map(|s| s.as_str()), Some("value"));
    }
}
