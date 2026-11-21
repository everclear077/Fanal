# Fanal 怎么读一个 Agent

[English](AGENTS.md)

Fanal 用两件事决定一盏灯的颜色：这个 Agent 的进程是不是在跑，以及它最新的会话文件最后说了什么。判断本身是纯函数，放在 `fanal-core` 里，所以不需要桌面会话就能测试。`fanal` 只负责把事实收集起来。

## 三种颜色

| 灯 | 含义 |
| --- | --- |
| 绿 | 进程在跑。如果一份新鲜的会话说它正在工作，这一行还会写出它在做什么。 |
| 黄 | 一份新鲜的会话停在了只有用户能回答的地方。 |
| 红 | 进程没有在跑。 |

超过 45 秒没有更新的会话读数会被忽略，灯就只跟着进程走。这样一个崩溃后留下来的文件不会把灯一直点成黄色。在设置里关掉的 Agent 直接不出现。

黄灯优先于绿灯，绿灯优先于红灯。托盘图标和收起来的刘海取整块面板上最高的那一种颜色。

## 三种读法

只有三个 Agent 的会话格式是拿真实文件核过的。其余的只看进程，而进程只能说明它在跑，别的什么都说不了。只靠进程点亮的绿灯没有详情一行，这是对“窗口开着”这件事最诚实的说法。

### Claude Code

`%USERPROFILE%\.claude\sessions\<pid>.json`。同时有多个会话时取最新的一份。只有 `status` 决定灯的颜色。

| `status` | 读数 |
| --- | --- |
| `busy`、`running` | 工作中 |
| `needs_approval`、`waiting`、`blocked` | 等待确认 |
| `idle`、`ended`、`stopped` | 空闲 |

`cwd` 的最后一段会变成详情一行。认不出的状态，或者不是合法 JSON 的文件，不会产生任何读数。

### Codex

`%USERPROFILE%\.codex\sessions\**\rollout-*.jsonl`。这个文件会一直长，所以只读最后 256 KB，并且丢掉落在中间的第一行。

`session_meta` 带着工作目录。`event_msg` 的载荷类型为 `task_started` 时一轮开始，`task_complete` 或 `turn_aborted` 时结束。没有结束的一轮读作工作中，结束了的读作空闲。

Codex 的审批策略是整个线程记一次，不是每一轮记一次。没有一个事件能可靠地说明某一轮正在等用户，所以 Codex 目前不会出现黄灯。

### Kimi Code

`%USERPROFILE%\.kimi-code\sessions\**\agents\main\wire.jsonl`，同样只读尾部。`agent.turn.started` 和 `agent.message.appended` 读作工作中，`agent.turn.ended` 读作空闲。

## 进程名

比对的是进程镜像名，统一转成小写并去掉 `.exe`。

| Agent | 镜像名 |
| --- | --- |
| Claude Code | `claude` |
| Cursor | `cursor` |
| DeepSeek Harness | `deepseek harness` |
| Codex | `codex`、`chatgpt` |
| Kimi | `kimi` |
| Kimi Code | `kimi code`、`kimi-code` |
| Orca | `orca` |
| ChatGPT | `chatgpt` |
| Gemini | `gemini` |
| Trae | `trae`、`trae cn`、`trae solo cn` |
| Kiro | `kiro` |
| Qoder | `qoder` |
| Antigravity | `antigravity` |
| OpenCode | `opencode` |
| Devin | `devin` |

Codex 和 ChatGPT 桌面版用的是同一个镜像名，所以 ChatGPT 在跑时 Codex 的灯也会亮。点击一行会把这个进程的窗口调到前台并最大化；如果它没在跑，就用当前进程的可执行文件路径把它启动起来。

## 故意不读的东西

会话文件里是对话本身。解析只取上面点到名的字段，其余的全部丢掉；除了那一行短短的详情，会话里的内容不会被记录、保存或展示。Fanal 自己的设置放在 `%APPDATA%\Fanal\settings.json`，从来不往任何一个 Agent 的目录里写东西。

## 加一个 Agent

1. 在 `fanal-core/src/model.rs` 的 `AgentId` 里加一个变体，写明 serde 重命名和显示名称。
2. 在 `builtin_catalog()` 里加一条 `AgentProfile`，写上要匹配的镜像名。
3. 在 `ui/marks/<id>.svg` 放一个图标，用 `tools/inline-marks.ps1` 重新生成 `ui/marks.js`。
4. 如果这个 Agent 会写会话文件，并且格式你拿真实文件核过，就在上面三个读取器旁边加一个，并用一段真实样本给它写测试。没核过的格式就继续只看进程。
