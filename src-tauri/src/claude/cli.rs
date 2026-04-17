// Claude CLI 进程封装
// 通过 tokio::process::Command 调用 claude 二进制

use std::process::Stdio;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::process::Command;

/// Claude CLI 版本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliInfo {
    pub available: bool,
    pub version: Option<String>,
    pub path: Option<String>,
}

/// 检查 claude CLI 是否可用，返回版本信息
pub async fn check_cli() -> CliInfo {
    match Command::new("claude").arg("--version").output().await {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            // 尝试获取完整路径
            let path = Command::new("which")
                .arg("claude")
                .output()
                .await
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

            CliInfo {
                available: true,
                version: Some(version),
                path,
            }
        }
        _ => CliInfo {
            available: false,
            version: None,
            path: None,
        },
    }
}

/// 向 Claude Code 发送单次提问并返回纯文本结果
pub async fn ask(prompt: &str, cwd: Option<&str>) -> Result<String> {
    let mut cmd = Command::new("claude");
    cmd.arg("--output-format")
        .arg("text")
        .arg("--print")
        .arg(prompt)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }

    let output = cmd
        .output()
        .await
        .context("Failed to execute claude CLI")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("claude exited with {}: {}", output.status, stderr.trim());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

