use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::Emitter;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::{oneshot, Mutex};

use super::permissions::PermissionRequest;

/// Hook 脚本发送的请求格式
#[derive(Debug, Deserialize)]
struct HookRequest {
    id: String,
    payload: serde_json::Value,
}

/// 返回给 hook 脚本的响应
#[derive(Debug, Serialize)]
struct HookResponse {
    decision: String,
}

/// 前端决策结果
#[derive(Debug, Clone)]
pub struct Decision {
    pub allow: bool,
}

/// 待处理请求的共享状态
/// key: request_id, value: oneshot sender
pub struct PendingRequests {
    senders: Mutex<HashMap<String, oneshot::Sender<Decision>>>,
}

impl PendingRequests {
    pub fn new() -> Self {
        Self {
            senders: Mutex::new(HashMap::new()),
        }
    }

    /// 注册一个待处理请求，返回 receiver 用于等待前端决策
    async fn register(&self, id: String) -> oneshot::Receiver<Decision> {
        let (tx, rx) = oneshot::channel();
        self.senders.lock().await.insert(id, tx);
        rx
    }

    /// 前端送回决策，唤醒等待的 socket handler
    pub async fn resolve(&self, id: &str, decision: Decision) -> bool {
        if let Some(sender) = self.senders.lock().await.remove(id) {
            let _ = sender.send(decision);
            true
        } else {
            tracing::warn!(request_id = id, "no pending request found for decision");
            false
        }
    }

    /// 清理超时的请求（取消等待）
    async fn remove(&self, id: &str) {
        self.senders.lock().await.remove(id);
    }
}

/// Socket 文件路径
pub fn socket_path() -> Option<PathBuf> {
    super::settings::signal_dir().map(|d| d.join("ipc.sock"))
}

/// 启动 Unix Domain Socket 服务器
/// 返回 JoinHandle 用于生命周期管理
pub async fn serve(
    app_handle: tauri::AppHandle,
    pending: Arc<PendingRequests>,
) -> anyhow::Result<()> {
    let sock_path = socket_path()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine socket path"))?;

    // 确保目录存在
    if let Some(parent) = sock_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // 清理旧 socket 文件
    let _ = std::fs::remove_file(&sock_path);

    let listener = UnixListener::bind(&sock_path)?;

    // 设置权限 0600（仅 owner 可读写）
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&sock_path, std::fs::Permissions::from_mode(0o600))?;
    }

    tracing::info!(?sock_path, "IPC socket server started");

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let app = app_handle.clone();
                let pending = pending.clone();
                let sock = sock_path.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_client(stream, app, pending).await {
                        tracing::warn!("IPC client error: {}", e);
                    }
                    // 确保 socket 文件还在（不会被客户端删掉）
                    let _ = &sock;
                });
            }
            Err(e) => {
                tracing::error!("IPC accept error: {}", e);
            }
        }
    }
}

async fn handle_client(
    stream: tokio::net::UnixStream,
    app: tauri::AppHandle,
    pending: Arc<PendingRequests>,
) -> anyhow::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);

    // 读取一行 JSON（hook 脚本用 printf + nc 发送，以换行结束）
    let mut line = String::new();
    buf_reader.read_line(&mut line).await?;
    let line = line.trim();

    if line.is_empty() {
        return Ok(());
    }

    let hook_req: HookRequest = serde_json::from_str(line)
        .map_err(|e| anyhow::anyhow!("Invalid hook request JSON: {}", e))?;

    let request_id = hook_req.id.clone();

    // 从 payload 中提取权限请求信息
    let tool_name = hook_req.payload
        .get("tool_name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let session_id = hook_req.payload
        .get("session_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let tool_input = hook_req.payload
        .get("tool_input")
        .map(|v| v.to_string());

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let perm_req = PermissionRequest {
        request_id: request_id.clone(),
        session_id,
        tool_name: tool_name.clone(),
        tool_input,
        timestamp: now,
    };

    tracing::info!(request_id = %request_id, tool = %tool_name, "IPC permission request received");

    // 注册等待 channel
    let rx = pending.register(request_id.clone()).await;

    // 发送 Tauri 事件给前端
    if let Err(e) = app.emit("permission-request", &perm_req) {
        tracing::error!("Failed to emit permission-request: {}", e);
        pending.remove(&request_id).await;
        // 发送 allow 作为 fallback
        let resp = serde_json::to_string(&HookResponse { decision: "allow".to_string() })?;
        writer.write_all(resp.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.shutdown().await?;
        return Ok(());
    }

    // 发送原生通知 + Dock badge
    crate::notifications::send_permission_notification(&app, &tool_name);

    // 同时写入审计文件（可选，便于调试）
    write_audit_file(&request_id, &perm_req);

    // 等待前端决策，120 秒超时
    let decision = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        rx,
    ).await;

    let decision_str = match decision {
        Ok(Ok(d)) => {
            crate::notifications::clear_one_permission();
            if d.allow { "allow" } else { "deny" }
        }
        Ok(Err(_)) => {
            crate::notifications::clear_one_permission();
            tracing::warn!(request_id = %request_id, "decision sender dropped, defaulting to allow");
            "allow"
        }
        Err(_) => {
            crate::notifications::clear_one_permission();
            tracing::warn!(request_id = %request_id, "decision timeout (120s), defaulting to ask");
            pending.remove(&request_id).await;
            "ask"
        }
    };

    tracing::info!(request_id = %request_id, decision = decision_str, "IPC permission response");

    let resp = serde_json::to_string(&HookResponse {
        decision: decision_str.to_string(),
    })?;
    writer.write_all(resp.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.shutdown().await?;

    Ok(())
}

/// 写入审计文件（可选，便于调试）
fn write_audit_file(request_id: &str, req: &PermissionRequest) {
    if let Some(req_dir) = super::settings::requests_dir() {
        let _ = std::fs::create_dir_all(&req_dir);
        let path = req_dir.join(format!("{}.json", request_id));
        if let Ok(json) = serde_json::to_string_pretty(req) {
            let _ = std::fs::write(&path, json);
        }
    }
}

/// 清理 socket 文件（应用退出时调用）
pub fn cleanup_socket() {
    if let Some(path) = socket_path() {
        let _ = std::fs::remove_file(&path);
        tracing::info!("IPC socket cleaned up");
    }
}
