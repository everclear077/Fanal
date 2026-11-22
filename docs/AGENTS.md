# How Fanal reads an agent

[中文](agents.zh-CN.md)

Fanal decides a lamp from two facts: whether the agent's process is running, and what its newest session file last said. The decision itself is pure and lives in `fanal-core`, so it can be tested without a desktop session. `fanal` only collects the facts.

## The three colours

| Lamp | Meaning |
| --- | --- |
| Grey | The program is installed and no process is running. |
| Yellow | The client is open and no task is executing. An open client with no session reading is yellow, because a process can only say that the window is open. |
| Green | A session written during this run says a task is executing. The row names what it is working on. |
| Red | A session written during this run says a task opened a confirmation and is waiting on the user. |

A session file last written before its process started is ignored, so a file left by an earlier run cannot keep a lamp red or green. While the process is still running, the last status stands even if the file is not rewritten. An agent the user has switched off in settings disappears entirely, and an agent added to the catalog after the settings file was saved stays off until it is switched on.

Red outranks green, green outranks yellow, and yellow outranks grey. The tray icon and the collapsed notch take the highest colour on the board.

## Three kinds of reading

Five agents have a session format that was checked against real files. Every other agent is read from its process, which can say that the window is open and nothing more. A process-only lamp is yellow and carries no detail line, which is the honest version of "the window is open".

### Claude Code and Claude Desktop

Both write `%USERPROFILE%\.claude\sessions\<pid>.json`. The desktop app keeps one directory per profile at `%LOCALAPPDATA%\Claude-3p\local-agent-mode-sessions\<account>\<profile>\.claude\sessions`, and every one of them is read. A newer directory can be empty while an older one still has a task running. One file per live process. When several are live, a confirmation outranks a running task, which outranks an idle session. `status` is the only field that decides the lamp.

| `status` | Reading |
| --- | --- |
| `busy`, `running` | Working |
| `needs_approval`, `waiting`, `blocked` | Needs confirmation |
| `idle`, `ended`, `stopped` | Idle |

The last segment of `cwd` becomes the detail line. A status Fanal does not recognise, or a file that is not valid JSON, yields no reading at all.

### Codex

`%USERPROFILE%\.codex\sessions\**\rollout-*.jsonl`. The file grows without bound, so only the last 256 KB is read and the partial first line is dropped.

`session_meta` carries the working directory. An `event_msg` whose payload type is `task_started` opens a turn; `task_complete` or `turn_aborted` closes it. An open turn reads as working, a closed one as idle.

Codex records its approval policy once, for the whole thread, rather than per turn. There is no event that reliably says a turn is waiting on the user, so Codex does not produce a red lamp yet. An open Codex with no running task is yellow.

### Kimi Code

`%USERPROFILE%\.kimi-code\sessions\**\agents\main\wire.jsonl`, again read from the tail. `agent.turn.started` and `agent.message.appended` read as working; `agent.turn.ended` reads as idle.

### Kimi

`%APPDATA%\kimi-desktop\kimi-agent\conversation-statuses.json`. Each entry is a status word. `running` reads as working, and a file whose entries are all `completed` reads as idle. The conversation ids are not shown. Nothing in this file says a task is waiting on the user, so the Kimi desktop lamp does not turn red.

## Process names

A process matches on its image name, compared in lowercase with `.exe` removed.

| Agent | Image names |
| --- | --- |
| Claude Code | `claude`, except the Store package |
| Claude Desktop | `claude` inside a `WindowsApps\Claude_<version>` package, or under `Claude-3p` |
| Cursor | `cursor` |
| DeepSeek Harness | `deepseek harness` |
| Codex | `chatgpt`, `codex` |
| Kimi | `kimi` |
| Kimi Code | `kimi code`, `kimi-code` |
| Gemini | `gemini` |
| Trae | `trae`, `trae cn`, `trae solo cn` |
| Kiro | `kiro` |
| Qoder | `qoder` |
| Antigravity | `antigravity` |
| OpenCode | `opencode` |
| Devin | `devin` |

The Codex app and the ChatGPT desktop app are one program. `chatgpt` owns the window and `codex` is its helper, so they share the Codex lamp. Clicking the row focuses that window. A closed program is launched from the path Windows recorded for it.

## What is deliberately not read

Session files hold the conversation. The parsers deserialise only the fields named above and drop everything else; nothing from a session is logged, stored, or shown except the short detail line. Fanal keeps its own settings in `%APPDATA%\Fanal\settings.json` and never writes inside an agent's directory.

## Adding an agent

1. Add a variant to `AgentId` in `fanal-core/src/model.rs`, with an explicit serde rename and a display name.
2. Add an `AgentProfile` in `builtin_catalog()` with the image names to match.
3. Add a mark at `ui/marks/<id>.svg` and regenerate `ui/marks.js` with `tools/inline-marks.ps1`.
4. If the agent writes a session file whose format you have verified, add a reader next to the three above and a test that feeds it a real sample. An unverified format should stay process-only.
