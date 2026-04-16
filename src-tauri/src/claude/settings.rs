use std::path::PathBuf;

use anyhow::Result;
use serde_json::Value;

const HOOK_MARKER: &str = "claude-pet";
const SIGNAL_FILE_NAME: &str = "waiting-signal";
const SCRIPT_FILE_NAME: &str = "notify-waiting.sh";
const PRETOOLUSE_SCRIPT_NAME: &str = "pretooluse-hook.sh";

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

/// 读取 ~/.claude/settings.json，不存在则返回空对象
fn read_claude_settings() -> Result<Value> {
    let path = claude_settings_path()
        .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;

    if !path.exists() {
        return Ok(serde_json::json!({}));
    }

    let content = std::fs::read_to_string(&path)?;
    let value: Value = serde_json::from_str(&content)?;
    Ok(value)
}

/// 写回 ~/.claude/settings.json（美化格式）
fn write_claude_settings(value: &Value) -> Result<()> {
    let path = claude_settings_path()
        .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;

    // 确保 ~/.claude/ 目录存在
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(value)?;
    std::fs::write(&path, content)?;
    Ok(())
}

/// 检查是否已安装 ClaudePet 的 Notification hook
pub fn is_hook_installed() -> Result<bool> {
    let settings = read_claude_settings()?;

    let has_hook = settings
        .get("hooks")
        .and_then(|h| h.get("Notification"))
        .and_then(|n| n.as_array())
        .map(|arr| {
            arr.iter().any(|entry| {
                entry
                    .get("hooks")
                    .and_then(|h| h.as_array())
                    .map(|hooks| {
                        hooks.iter().any(|hook| {
                            hook.get("command")
                                .and_then(|c| c.as_str())
                                .map(|s| s.contains(HOOK_MARKER))
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);

    Ok(has_hook)
}

/// 安装 Notification hook：创建脚本 + 注入 settings.json
pub fn install_hook() -> Result<()> {
    // 1. 创建信号目录和脚本
    let dir = signal_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;
    std::fs::create_dir_all(&dir)?;

    let script_path = signal_script_path()
        .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;

    let signal_file = signal_file_path()
        .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;

    let script_content = format!(
        "#!/bin/bash\n# ClaudePet notification hook\ndate +%s > \"{}\"\n",
        signal_file.display()
    );
    std::fs::write(&script_path, &script_content)?;

    // chmod +x
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))?;
    }

    // 2. 注入 hook 配置到 settings.json
    let mut settings = read_claude_settings()?;

    let hook_entry = serde_json::json!({
        "matcher": "permission_prompt",
        "hooks": [{
            "type": "command",
            "command": script_path.display().to_string()
        }]
    });

    // 确保 hooks.Notification 数组存在
    let hooks = settings
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("settings.json root is not an object"))?
        .entry("hooks")
        .or_insert_with(|| serde_json::json!({}));

    let notification = hooks
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("hooks is not an object"))?
        .entry("Notification")
        .or_insert_with(|| serde_json::json!([]));

    let arr = notification
        .as_array_mut()
        .ok_or_else(|| anyhow::anyhow!("Notification is not an array"))?;

    // 移除旧的 ClaudePet hook（如果存在）
    arr.retain(|entry| {
        !entry
            .get("hooks")
            .and_then(|h| h.as_array())
            .map(|hooks| {
                hooks.iter().any(|hook| {
                    hook.get("command")
                        .and_then(|c| c.as_str())
                        .map(|s| s.contains(HOOK_MARKER))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false)
    });

    // 添加新条目
    arr.push(hook_entry);

    write_claude_settings(&settings)?;
    log::info!("Notification hook installed");
    Ok(())
}

/// 卸载 Notification hook：移除配置 + 删除脚本
pub fn uninstall_hook() -> Result<()> {
    // 1. 从 settings.json 移除 hook
    let mut settings = read_claude_settings()?;

    if let Some(arr) = settings
        .get_mut("hooks")
        .and_then(|h| h.get_mut("Notification"))
        .and_then(|n| n.as_array_mut())
    {
        arr.retain(|entry| {
            !entry
                .get("hooks")
                .and_then(|h| h.as_array())
                .map(|hooks| {
                    hooks.iter().any(|hook| {
                        hook.get("command")
                            .and_then(|c| c.as_str())
                            .map(|s| s.contains(HOOK_MARKER))
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false)
        });

        write_claude_settings(&settings)?;
    }

    // 2. 删除脚本和信号文件
    if let Some(script) = signal_script_path() {
        let _ = std::fs::remove_file(&script);
    }
    if let Some(signal) = signal_file_path() {
        let _ = std::fs::remove_file(&signal);
    }

    log::info!("Notification hook uninstalled");
    Ok(())
}

/// 检查 PreToolUse hook 是否已安装
pub fn is_pretooluse_hook_installed() -> Result<bool> {
    let settings = read_claude_settings()?;

    let has_hook = settings
        .get("hooks")
        .and_then(|h| h.get("PreToolUse"))
        .and_then(|n| n.as_array())
        .map(|arr| {
            arr.iter().any(|entry| {
                entry
                    .get("hooks")
                    .and_then(|h| h.as_array())
                    .map(|hooks| {
                        hooks.iter().any(|hook| {
                            hook.get("command")
                                .and_then(|c| c.as_str())
                                .map(|s| s.contains(HOOK_MARKER))
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);

    Ok(has_hook)
}

/// 安装 PreToolUse hook：创建脚本 + 注入 settings.json
pub fn install_pretooluse_hook() -> Result<()> {
    let dir = signal_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;
    std::fs::create_dir_all(&dir)?;

    // 确保 requests / responses 目录存在
    if let Some(req_dir) = requests_dir() {
        std::fs::create_dir_all(&req_dir)?;
    }
    if let Some(resp_dir) = responses_dir() {
        std::fs::create_dir_all(&resp_dir)?;
    }

    let script_path = pretooluse_script_path()
        .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;

    let req_dir = requests_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;
    let resp_dir = responses_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;
    let intercept_flag = dir.join("intercept-active");

    let script_content = format!(
        r#"#!/bin/bash
# claude-pet PreToolUse hook
INPUT=$(cat)
REQUEST_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')

# 检查 ClaudePet 是否在拦截模式
if [ ! -f "{intercept_flag}" ]; then
  echo '{{"hookSpecificOutput":{{"hookEventName":"PreToolUse","permissionDecision":"ask"}}}}'
  exit 0
fi

# 提取工具信息
TOOL_INFO=$(/usr/bin/python3 -c "
import sys, json, time
d = json.load(sys.stdin)
out = {{
  'requestId': '$REQUEST_ID',
  'sessionId': d.get('session_id', ''),
  'toolName': d.get('tool_name', 'unknown'),
  'toolInput': json.dumps(d.get('tool_input', {{}}))[:200],
  'timestamp': int(time.time() * 1000)
}}
print(json.dumps(out))
" <<< "$INPUT" 2>/dev/null)

if [ -z "$TOOL_INFO" ]; then
  echo '{{"hookSpecificOutput":{{"hookEventName":"PreToolUse","permissionDecision":"ask"}}}}'
  exit 0
fi

# 写入请求文件
mkdir -p "{req_dir}"
echo "$TOOL_INFO" > "{req_dir}/$REQUEST_ID.json"

# 等待响应（poll 200ms，超时 120s）
RESPONSE="{resp_dir}/$REQUEST_ID.json"
for i in $(seq 1 600); do
  if [ -f "$RESPONSE" ]; then
    DECISION=$(/usr/bin/python3 -c "
import json
d = json.load(open('$RESPONSE'))
print(d.get('decision', 'ask'))
" 2>/dev/null || echo "ask")
    rm -f "$RESPONSE" "{req_dir}/$REQUEST_ID.json"
    echo "{{"hookSpecificOutput":{{"hookEventName":"PreToolUse","permissionDecision":"$DECISION","permissionDecisionReason":"ClaudePet user decision"}}}}"
    exit 0
  fi
  sleep 0.2
done

# 超时 → ask
rm -f "{req_dir}/$REQUEST_ID.json"
echo '{{"hookSpecificOutput":{{"hookEventName":"PreToolUse","permissionDecision":"ask"}}}}'
"#,
        intercept_flag = intercept_flag.display(),
        req_dir = req_dir.display(),
        resp_dir = resp_dir.display(),
    );

    std::fs::write(&script_path, &script_content)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))?;
    }

    // 注入 hook 配置到 settings.json
    let mut settings = read_claude_settings()?;

    let hook_entry = serde_json::json!({
        "matcher": "",
        "hooks": [{
            "type": "command",
            "command": script_path.display().to_string()
        }]
    });

    let hooks = settings
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("settings.json root is not an object"))?
        .entry("hooks")
        .or_insert_with(|| serde_json::json!({}));

    let pretooluse = hooks
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("hooks is not an object"))?
        .entry("PreToolUse")
        .or_insert_with(|| serde_json::json!([]));

    let arr = pretooluse
        .as_array_mut()
        .ok_or_else(|| anyhow::anyhow!("PreToolUse is not an array"))?;

    // 移除旧的 ClaudePet hook
    arr.retain(|entry| {
        !entry
            .get("hooks")
            .and_then(|h| h.as_array())
            .map(|hooks| {
                hooks.iter().any(|hook| {
                    hook.get("command")
                        .and_then(|c| c.as_str())
                        .map(|s| s.contains(HOOK_MARKER))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false)
    });

    arr.push(hook_entry);

    write_claude_settings(&settings)?;
    log::info!("PreToolUse hook installed");
    Ok(())
}

/// 卸载 PreToolUse hook
pub fn uninstall_pretooluse_hook() -> Result<()> {
    let mut settings = read_claude_settings()?;

    if let Some(arr) = settings
        .get_mut("hooks")
        .and_then(|h| h.get_mut("PreToolUse"))
        .and_then(|n| n.as_array_mut())
    {
        arr.retain(|entry| {
            !entry
                .get("hooks")
                .and_then(|h| h.as_array())
                .map(|hooks| {
                    hooks.iter().any(|hook| {
                        hook.get("command")
                            .and_then(|c| c.as_str())
                            .map(|s| s.contains(HOOK_MARKER))
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false)
        });

        write_claude_settings(&settings)?;
    }

    // 删除脚本和标志文件
    if let Some(script) = pretooluse_script_path() {
        let _ = std::fs::remove_file(&script);
    }

    log::info!("PreToolUse hook uninstalled");
    Ok(())
}
