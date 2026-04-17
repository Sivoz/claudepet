use std::path::{Path, PathBuf};

use anyhow::Result;
use serde_json::Value;

const SIGNAL_FILE_NAME: &str = "waiting-signal";
const SCRIPT_FILE_NAME: &str = "notify-waiting.sh";
const PRETOOLUSE_SCRIPT_NAME: &str = "pretooluse-hook.sh";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const MAX_BACKUPS: usize = 3;

// ---- 路径辅助 ----

/// ~/.claude/settings.json 路径
fn claude_settings_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude").join("settings.json"))
}

/// ~/.claude-pet/ 目录
pub fn signal_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude-pet"))
}

/// 信号文件路径
pub fn signal_file_path() -> Option<PathBuf> {
    signal_dir().map(|d| d.join(SIGNAL_FILE_NAME))
}

/// Hook 脚本路径
fn signal_script_path() -> Option<PathBuf> {
    signal_dir().map(|d| d.join(SCRIPT_FILE_NAME))
}

/// PreToolUse hook 脚本路径
fn pretooluse_script_path() -> Option<PathBuf> {
    signal_dir().map(|d| d.join(PRETOOLUSE_SCRIPT_NAME))
}

/// 权限请求目录
pub fn requests_dir() -> Option<PathBuf> {
    signal_dir().map(|d| d.join("requests"))
}

/// 权限响应目录
pub fn responses_dir() -> Option<PathBuf> {
    signal_dir().map(|d| d.join("responses"))
}

// ---- 原子写入 + 备份 ----

/// 原子写入：先写 .tmp 再 rename，防止读到半状态
fn atomic_write(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, data)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// 备份 settings.json，保留最近 MAX_BACKUPS 份
fn backup_settings(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let parent = path.parent().unwrap_or(Path::new("."));
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let backup_name = format!("settings.json.claudepet.bak.{}", ts);
    let backup_path = parent.join(&backup_name);

    std::fs::copy(path, &backup_path)?;
    tracing::info!(?backup_path, "settings.json backed up");

    // 清理旧备份
    let mut backups: Vec<_> = std::fs::read_dir(parent)?
        .flatten()
        .filter(|e| {
            e.file_name()
                .to_str()
                .map(|n| n.starts_with("settings.json.claudepet.bak."))
                .unwrap_or(false)
        })
        .collect();

    if backups.len() > MAX_BACKUPS {
        backups.sort_by_key(|e| e.file_name());
        for old in &backups[..backups.len() - MAX_BACKUPS] {
            let _ = std::fs::remove_file(old.path());
            tracing::debug!(path = ?old.path(), "old backup removed");
        }
    }

    Ok(())
}

// ---- settings.json 读写 ----

/// 读取 ~/.claude/settings.json，不存在则返回空对象
fn read_claude_settings() -> Result<Value> {
    let path = claude_settings_path()
        .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;

    if !path.exists() {
        return Ok(serde_json::json!({}));
    }

    let content = std::fs::read(&path)?;
    let value: Value = serde_json::from_slice(&content)
        .map_err(|e| anyhow::anyhow!("settings.json is not valid JSON: {}", e))?;
    Ok(value)
}

/// 写回 ~/.claude/settings.json（原子写入，写前备份）
fn write_claude_settings(value: &Value) -> Result<()> {
    let path = claude_settings_path()
        .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // 写前备份
    backup_settings(&path)?;

    // 序列化并验证
    let content = serde_json::to_string_pretty(value)?;
    // 二次验证：确保我们写出的是合法 JSON
    serde_json::from_str::<Value>(&content)?;

    atomic_write(&path, content.as_bytes())?;
    Ok(())
}

// ---- managed 标记辅助 ----

/// 判断一个 hook 条目是否带有 `_claudepet_managed` 标记
fn is_managed(entry: &Value) -> bool {
    entry
        .get("_claudepet_managed")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

/// 构造带 managed 标记的 hook 条目
fn make_managed_entry(matcher: &str, script_path: &Path) -> Value {
    serde_json::json!({
        "_claudepet_managed": true,
        "_claudepet_version": APP_VERSION,
        "matcher": matcher,
        "hooks": [{
            "type": "command",
            "command": script_path.display().to_string()
        }]
    })
}

/// 从 hook 数组中移除所有 managed 条目
fn remove_managed_entries(arr: &mut Vec<Value>) {
    arr.retain(|entry| !is_managed(entry));
}

/// 从 hook 条目中提取脚本路径
fn extract_script_path(entry: &Value) -> Option<String> {
    entry
        .get("hooks")
        .and_then(|h| h.as_array())
        .and_then(|hooks| hooks.first())
        .and_then(|hook| hook.get("command"))
        .and_then(|c| c.as_str())
        .map(|s| s.to_string())
}

// ---- Hook 状态 ----

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HookHealth {
    pub notification: HookEntryHealth,
    pub pre_tool_use: HookEntryHealth,
    pub settings_valid: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HookEntryHealth {
    /// 已注入且脚本存在
    Healthy,
    /// 未注入（用户未安装或主动禁用）
    Inactive,
    /// 注入条目存在但脚本缺失
    Broken,
}

// ---- 公共 API ----

/// 检查是否已安装 ClaudePet 的 Notification hook
pub fn is_hook_installed() -> Result<bool> {
    let settings = read_claude_settings()?;
    Ok(has_managed_hook(&settings, "Notification"))
}

/// 检查 PreToolUse hook 是否已安装
pub fn is_pretooluse_hook_installed() -> Result<bool> {
    let settings = read_claude_settings()?;
    Ok(has_managed_hook(&settings, "PreToolUse"))
}

fn has_managed_hook(settings: &Value, hook_type: &str) -> bool {
    settings
        .get("hooks")
        .and_then(|h| h.get(hook_type))
        .and_then(|n| n.as_array())
        .map(|arr| arr.iter().any(|entry| is_managed(entry)))
        .unwrap_or(false)
}

/// 安装 Notification hook：创建脚本 + 注入 settings.json
pub fn install_hook() -> Result<()> {
    let dir = signal_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;
    std::fs::create_dir_all(&dir)?;

    let script_path = signal_script_path()
        .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;
    let signal_file = signal_file_path()
        .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;

    let script_content = format!(
        "#!/bin/bash\n# ClaudePet notification hook\nLOG_DIR=\"$HOME/.claude-pet/logs\"\nmkdir -p \"$LOG_DIR\"\nexec 2>>\"$LOG_DIR/hook.log\"\necho \"[$(date -u +%%FT%%TZ)] Notification hook invoked\" >&2\ndate +%s > \"{}\"\n",
        signal_file.display()
    );
    std::fs::write(&script_path, &script_content)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))?;
    }

    let mut settings = read_claude_settings()?;
    let hook_entry = make_managed_entry("permission_prompt", &script_path);

    inject_hook(&mut settings, "Notification", hook_entry)?;
    write_claude_settings(&settings)?;
    tracing::info!("Notification hook installed");
    Ok(())
}

/// 卸载 Notification hook：移除配置 + 删除脚本
pub fn uninstall_hook() -> Result<()> {
    let mut settings = read_claude_settings()?;

    if remove_hook(&mut settings, "Notification") {
        write_claude_settings(&settings)?;
    }

    if let Some(script) = signal_script_path() {
        let _ = std::fs::remove_file(&script);
    }
    if let Some(signal) = signal_file_path() {
        let _ = std::fs::remove_file(&signal);
    }

    tracing::info!("Notification hook uninstalled");
    Ok(())
}

/// 安装 PreToolUse hook：创建脚本 + 注入 settings.json
pub fn install_pretooluse_hook() -> Result<()> {
    let dir = signal_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;
    std::fs::create_dir_all(&dir)?;

    if let Some(req_dir) = requests_dir() {
        std::fs::create_dir_all(&req_dir)?;
    }
    if let Some(resp_dir) = responses_dir() {
        std::fs::create_dir_all(&resp_dir)?;
    }

    let script_path = pretooluse_script_path()
        .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;
    let intercept_flag = dir.join("intercept-active");
    let sock_path = dir.join("ipc.sock");

    let script_content = format!(
        r#"#!/bin/bash
# claude-pet PreToolUse hook — Unix Domain Socket IPC
LOG_DIR="$HOME/.claude-pet/logs"
mkdir -p "$LOG_DIR"
exec 2>>"$LOG_DIR/hook.log"
echo "[$(date -u +%FT%TZ)] PreToolUse hook invoked" >&2

SOCK="{sock_path}"

# 拦截开关
if [ ! -f "{intercept_flag}" ]; then
  exit 0
fi

# Socket 不存在则放行（ClaudePet 未运行）
if [ ! -S "$SOCK" ]; then
  echo "[$(date -u +%FT%TZ)] [warn] socket missing, pass-through" >&2
  exit 0
fi

INPUT=$(cat)
REQUEST_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')
echo "[$(date -u +%FT%TZ)] request=$REQUEST_ID" >&2

# 通过 socket 发送请求并等待响应
# printf 发送一行 JSON，nc -U 连接 socket，timeout 125 秒
RESP=$(printf '{{"id":"%s","payload":%s}}\n' "$REQUEST_ID" "$INPUT" \
  | timeout 125 nc -U "$SOCK" 2>/dev/null)

if [ -z "$RESP" ]; then
  echo "[$(date -u +%FT%TZ)] [warn] no response from socket, pass-through" >&2
  exit 0
fi

echo "[$(date -u +%FT%TZ)] response=$RESP" >&2

# 解析 decision
case "$RESP" in
  *'"allow"'*) DECISION="allow" ;;
  *'"deny"'*)  DECISION="deny" ;;
  *)           DECISION="ask" ;;
esac

if [ "$DECISION" = "deny" ]; then
  echo '{{"hookSpecificOutput":{{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"ClaudePet user denied"}}}}'
  exit 2
fi

if [ "$DECISION" = "allow" ]; then
  echo '{{"hookSpecificOutput":{{"hookEventName":"PreToolUse","permissionDecision":"allow","permissionDecisionReason":"ClaudePet user approved"}}}}'
  exit 0
fi

# ask = 让 Claude Code 自行处理
exit 0
"#,
        sock_path = sock_path.display(),
        intercept_flag = intercept_flag.display(),
    );

    std::fs::write(&script_path, &script_content)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))?;
    }

    let mut settings = read_claude_settings()?;
    let hook_entry = make_managed_entry("", &script_path);

    inject_hook(&mut settings, "PreToolUse", hook_entry)?;
    write_claude_settings(&settings)?;
    tracing::info!("PreToolUse hook installed");
    Ok(())
}

/// 卸载 PreToolUse hook
pub fn uninstall_pretooluse_hook() -> Result<()> {
    let mut settings = read_claude_settings()?;

    if remove_hook(&mut settings, "PreToolUse") {
        write_claude_settings(&settings)?;
    }

    if let Some(script) = pretooluse_script_path() {
        let _ = std::fs::remove_file(&script);
    }

    tracing::info!("PreToolUse hook uninstalled");
    Ok(())
}

/// 启动时自检：检查 managed hook 的脚本完整性
/// 脚本不存在 → 从 settings 移除孤儿条目
/// 版本过旧 → 记录 warn（不自动重注入，避免覆盖用户修改）
pub fn verify_hook_integrity() -> HookHealth {
    let settings_valid = match read_claude_settings() {
        Ok(mut settings) => {
            let mut dirty = false;
            for hook_type in &["Notification", "PreToolUse"] {
                if let Some(arr) = settings
                    .get_mut("hooks")
                    .and_then(|h| h.get_mut(*hook_type))
                    .and_then(|n| n.as_array_mut())
                {
                    let before = arr.len();
                    arr.retain(|entry| {
                        if !is_managed(entry) {
                            return true; // 保留非 managed
                        }
                        if let Some(script) = extract_script_path(entry) {
                            if Path::new(&script).exists() {
                                // 检查版本
                                let ver = entry
                                    .get("_claudepet_version")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown");
                                if ver != APP_VERSION {
                                    tracing::warn!(
                                        hook_type, ver, current = APP_VERSION,
                                        "managed hook version mismatch"
                                    );
                                }
                                true
                            } else {
                                tracing::warn!(
                                    hook_type, ?script,
                                    "orphan hook: script missing, removing entry"
                                );
                                false
                            }
                        } else {
                            tracing::warn!(hook_type, "managed hook without script path, removing");
                            false
                        }
                    });
                    if arr.len() != before {
                        dirty = true;
                    }
                }
            }
            if dirty {
                if let Err(e) = write_claude_settings(&settings) {
                    tracing::error!("Failed to write cleaned settings: {}", e);
                }
            }
            true
        }
        Err(e) => {
            tracing::error!("Cannot read settings.json for integrity check: {}", e);
            false
        }
    };

    let notification = check_entry_health("Notification");
    let pre_tool_use = check_entry_health("PreToolUse");

    let health = HookHealth {
        notification,
        pre_tool_use,
        settings_valid,
    };
    tracing::info!(?health, "Hook integrity check completed");
    health
}

fn check_entry_health(hook_type: &str) -> HookEntryHealth {
    let settings = match read_claude_settings() {
        Ok(s) => s,
        Err(_) => return HookEntryHealth::Broken,
    };

    let entries = settings
        .get("hooks")
        .and_then(|h| h.get(hook_type))
        .and_then(|n| n.as_array());

    let Some(arr) = entries else {
        return HookEntryHealth::Inactive;
    };

    let managed: Vec<_> = arr.iter().filter(|e| is_managed(e)).collect();
    if managed.is_empty() {
        return HookEntryHealth::Inactive;
    }

    for entry in &managed {
        if let Some(script) = extract_script_path(entry) {
            if !Path::new(&script).exists() {
                return HookEntryHealth::Broken;
            }
        } else {
            return HookEntryHealth::Broken;
        }
    }

    HookEntryHealth::Healthy
}

// ---- 内部辅助 ----

/// 注入 hook 条目到指定的 hook 类型数组
fn inject_hook(settings: &mut Value, hook_type: &str, entry: Value) -> Result<()> {
    let hooks = settings
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("settings.json root is not an object"))?
        .entry("hooks")
        .or_insert_with(|| serde_json::json!({}));

    let arr = hooks
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("hooks is not an object"))?
        .entry(hook_type)
        .or_insert_with(|| serde_json::json!([]));

    let arr = arr
        .as_array_mut()
        .ok_or_else(|| anyhow::anyhow!("{} is not an array", hook_type))?;

    // 移除旧的 managed 条目
    remove_managed_entries(arr);

    arr.push(entry);
    Ok(())
}

/// 从指定 hook 类型中移除所有 managed 条目，返回是否有改动
fn remove_hook(settings: &mut Value, hook_type: &str) -> bool {
    if let Some(arr) = settings
        .get_mut("hooks")
        .and_then(|h| h.get_mut(hook_type))
        .and_then(|n| n.as_array_mut())
    {
        let before = arr.len();
        remove_managed_entries(arr);
        arr.len() != before
    } else {
        false
    }
}
