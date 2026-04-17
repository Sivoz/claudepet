<p align="center">
  <img src="src-tauri/icons/icon.png" width="128" alt="ClaudePet" />
</p>

<h1 align="center">ClaudePet</h1>

<p align="center">
  <b>你的 Claude Code 桌面像素伙伴</b><br/>
  <sub>实时感知 Claude Code 工作状态，陪你一起写代码</sub>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Tauri-2-blue?logo=tauri" alt="Tauri 2" />
  <img src="https://img.shields.io/badge/Vue-3-42b883?logo=vuedotjs" alt="Vue 3" />
  <img src="https://img.shields.io/badge/Rust-ed8936?logo=rust&logoColor=white" alt="Rust" />
  <img src="https://img.shields.io/badge/macOS-supported-lightgrey?logo=apple" alt="macOS" />
</p>

---

## What is ClaudePet?

ClaudePet 是一个悬浮在所有窗口之上的桌面像素宠物。它通过监听 `~/.claude/projects/` 下的 JSONL 活动日志，实时反映 Claude Code 的工作状态 — 思考、编码、完成、出错、休眠 — 用可爱的像素动画让你随时知道 Claude 在做什么。

```
  thinking...    coding...     done!       sleeping...
    💭             ⌨️            ✨           💤
   (o.o)         (>.<)         (^▽^)        (-.-)
   /|  |\        /|  |\        /|  |\       /|  |\
```

## Features

**🎮 8 套手绘像素皮肤** — 全部由程序化 Canvas 2D 逐像素绘制，48×48 无锯齿放大

| 皮肤 | 风格 | 皮肤 | 风格 |
|:---:|:---:|:---:|:---:|
| **Sakura** `mahou` | 魔法少女 | **Devil** `devil` | 小恶魔 |
| **Neko** `neko` | 猫咪 | **Elf** `elf` | 精灵 |
| **Kitsune** `kitsune` | 狐狸 | **Yami** `yami` | 暗影 |
| **Usagi** `usagi` | 兔兔 | **Nexus** `nexus` | 赛博 |

**👀 实时状态感知**

| Claude Code 行为 | 宠物状态 | 视觉效果 |
|:---|:---|:---|
| 发送 prompt | 🔵 思考中 | 蓝色气泡 + 旋转光点 |
| 调用工具 / 生成代码 | 🟢 编码中 | 绿色代码括号 |
| 回复结束 (`end_turn` / `max_tokens`) | 🟡 已完成 | 金色星光（3s 后回到空闲） |
| 工具报错 | 🔴 出错了 | 红色闪烁 |
| 等待审批 | 🟠 等待中 | 橙色光晕 |
| 5 分钟无活动 | 🟣 休眠中 | 紫色 Z 字符 |
| 30 秒无新事件（异常中断） | ⚪ 自动回空闲 | Watchdog 超时兜底 |

**🖥️ 多屏 & 多 Space 支持** — 自动跟随鼠标所在屏幕，四指滑动切换桌面时始终可见

**📋 多会话管理** — 同时追踪多个 Claude Code 会话，独立显示每个会话的状态

**🔐 权限审批** — 宠物下方快捷审批 / 拒绝 Claude Code 的工具调用请求

## Quick Start

### 环境要求

- **Node.js** >= 18 + **pnpm**
- **Rust** (stable) + **Cargo**
- **Tauri 2 CLI** — `pnpm add -g @tauri-apps/cli`
- macOS (当前主要支持平台)

### 安装 & 运行

```bash
# 克隆仓库
git clone https://github.com/your-username/ClaudePet.git
cd ClaudePet

# 安装前端依赖
pnpm install

# 开发模式（Vite HMR + Tauri 窗口）
pnpm tauri dev

# 生产构建
pnpm tauri build
```

## Architecture

```
~/.claude/projects/**/*.jsonl          ← Claude Code 活动日志（只读）
         │
         ▼
┌─────────────────────────────┐
│  Rust Backend (Tauri 2)     │
│  ┌───────────────────────┐  │
│  │ notify + debouncer    │  │  ← 文件监听（300ms 防抖）
│  │ IncrementalParser     │  │  ← 增量解析（字节偏移 + 半行缓冲）
│  │ resolve_state()       │  │  ← JSONL → PetState 映射（stop_reason 感知）
│  │ SessionManager        │  │  ← 多会话追踪
│  │ Watchdog (tokio)      │  │  ← 30s 活跃状态超时兜底
│  └───────────┬───────────┘  │
│              │ emit          │
└──────────────┼──────────────┘
               ▼
┌─────────────────────────────┐
│  Vue 3 Frontend             │
│  ┌───────────────────────┐  │
│  │ useClaudeEvents       │  │  ← 事件监听 → Pinia store + 35s 前端兜底
│  │ useClaudeState        │  │  ← 休眠计时（5min idle → sleeping）
│  └───────────────────────┘  │
└─────────────────────────────┘
```

## Tech Stack

| 层 | 技术 |
|:---|:---|
| 桌面框架 | Tauri 2 |
| 后端 | Rust + Tokio |
| 前端 | Vue 3 + TypeScript + Pinia |
| 样式 | UnoCSS |
| 构建 | Vite 6 |
| 规范 | Conventional Commits (commitlint + husky) |

## License

MIT

---

<p align="center">
  <sub>Made with pixels and love 🩷</sub>
</p>
