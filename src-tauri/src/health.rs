use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct HealthReport {
    pub items: Vec<HealthItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthItem {
    pub name: String,
    pub status: HealthStatus,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Ok,
    Warn,
    Error,
}

pub fn check_all() -> HealthReport {
    let mut items = Vec::new();

    // 1. Claude CLI
    items.push(check_claude_cli());

    // 2. settings.json
    items.push(check_settings_json());

    // 3. Hook 完整性
    items.push(check_hooks());

    // 4. IPC socket
    items.push(check_ipc_socket());

    // 5. 日志目录
    items.push(check_log_dir());

    // 6. 配置文件
    items.push(check_config());

    HealthReport { items }
}

fn check_claude_cli() -> HealthItem {
    let result = std::process::Command::new("which")
        .arg("claude")
        .output();

    match result {
        Ok(output) if output.status.success() => {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            HealthItem {
                name: "Claude CLI".to_string(),
                status: HealthStatus::Ok,
                detail: path,
            }
        }
        _ => HealthItem {
            name: "Claude CLI".to_string(),
            status: HealthStatus::Warn,
            detail: "claude 不在 PATH 中".to_string(),
        },
    }
}

fn check_settings_json() -> HealthItem {
    let path = dirs::home_dir()
        .map(|h| h.join(".claude").join("settings.json"));

    let Some(path) = path else {
        return HealthItem {
            name: "settings.json".to_string(),
            status: HealthStatus::Error,
            detail: "无法确定 home 目录".to_string(),
        };
    };

    if !path.exists() {
        return HealthItem {
            name: "settings.json".to_string(),
            status: HealthStatus::Warn,
            detail: "文件不存在（首次使用）".to_string(),
        };
    }

    match std::fs::read(&path) {
        Ok(data) => match serde_json::from_slice::<serde_json::Value>(&data) {
            Ok(_) => HealthItem {
                name: "settings.json".to_string(),
                status: HealthStatus::Ok,
                detail: "JSON 合法，可读写".to_string(),
            },
            Err(e) => HealthItem {
                name: "settings.json".to_string(),
                status: HealthStatus::Error,
                detail: format!("JSON 解析失败: {}", e),
            },
        },
        Err(e) => HealthItem {
            name: "settings.json".to_string(),
            status: HealthStatus::Error,
            detail: format!("读取失败: {}", e),
        },
    }
}

fn check_hooks() -> HealthItem {
    let health = crate::claude::settings::verify_hook_integrity();

    use crate::claude::settings::HookEntryHealth;
    let n_status = match health.notification {
        HookEntryHealth::Healthy => "正常",
        HookEntryHealth::Inactive => "未安装",
        HookEntryHealth::Broken => "异常",
    };
    let p_status = match health.pre_tool_use {
        HookEntryHealth::Healthy => "正常",
        HookEntryHealth::Inactive => "未安装",
        HookEntryHealth::Broken => "异常",
    };

    let any_broken = matches!(health.notification, HookEntryHealth::Broken)
        || matches!(health.pre_tool_use, HookEntryHealth::Broken);

    HealthItem {
        name: "Hook 状态".to_string(),
        status: if any_broken {
            HealthStatus::Error
        } else {
            HealthStatus::Ok
        },
        detail: format!("Notification: {} | PreToolUse: {}", n_status, p_status),
    }
}

fn check_ipc_socket() -> HealthItem {
    let path = crate::claude::ipc::socket_path();
    match path {
        Some(p) if p.exists() => HealthItem {
            name: "IPC Socket".to_string(),
            status: HealthStatus::Ok,
            detail: p.display().to_string(),
        },
        Some(_) => HealthItem {
            name: "IPC Socket".to_string(),
            status: HealthStatus::Warn,
            detail: "socket 文件不存在".to_string(),
        },
        None => HealthItem {
            name: "IPC Socket".to_string(),
            status: HealthStatus::Error,
            detail: "无法确定路径".to_string(),
        },
    }
}

fn check_log_dir() -> HealthItem {
    match crate::logging::log_dir() {
        Ok(dir) => {
            if dir.exists() {
                HealthItem {
                    name: "日志目录".to_string(),
                    status: HealthStatus::Ok,
                    detail: dir.display().to_string(),
                }
            } else {
                HealthItem {
                    name: "日志目录".to_string(),
                    status: HealthStatus::Warn,
                    detail: "目录不存在".to_string(),
                }
            }
        }
        Err(e) => HealthItem {
            name: "日志目录".to_string(),
            status: HealthStatus::Error,
            detail: format!("{}", e),
        },
    }
}

fn check_config() -> HealthItem {
    let path = dirs::home_dir()
        .map(|h| h.join(".claude-pet").join("config.toml"));

    match path {
        Some(p) if p.exists() => {
            match std::fs::read_to_string(&p) {
                Ok(content) => match toml::from_str::<crate::config::AppConfig>(&content) {
                    Ok(_) => HealthItem {
                        name: "config.toml".to_string(),
                        status: HealthStatus::Ok,
                        detail: "配置合法".to_string(),
                    },
                    Err(e) => HealthItem {
                        name: "config.toml".to_string(),
                        status: HealthStatus::Error,
                        detail: format!("解析失败: {}", e),
                    },
                },
                Err(e) => HealthItem {
                    name: "config.toml".to_string(),
                    status: HealthStatus::Error,
                    detail: format!("读取失败: {}", e),
                },
            }
        }
        _ => HealthItem {
            name: "config.toml".to_string(),
            status: HealthStatus::Warn,
            detail: "使用默认配置".to_string(),
        },
    }
}
