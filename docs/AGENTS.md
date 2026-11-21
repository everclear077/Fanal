# How Fanal reads an agent

[中文](agents.zh-CN.md)

Fanal decides a lamp from two facts: whether the agent's process is running, and what its newest session file last said. The decision itself is pure and lives in `fanal-core`, so it can be tested without a desktop session. `fanal` only collects the facts.

## The three colours

| Lamp | Meaning |
| --- | --- |
| Green | The process is running. When a fresh session says it is working, the row also names what it is working on. |
| Amber | A fresh session stopped on something only the user can answer. |
| Red | The process is not running. |

A session reading older than 45 seconds is ignored. The lamp then follows the process alone, so a file left behind by a crashed session cannot keep a lamp amber. An agent the user has switched off in settings disappears entirely.

Amber outranks green and green outranks red. The tray icon and the collapsed notch take the highest colour on the board.

## Three kinds of reading

Only three agents have a session format that was checked against real files. Every other agent is read from its process, which can say that it is running and nothing more. A process-only green carries no detail line, which is the honest version of "the window is open".

### Claude Code

`%USERPROFILE%\.claude\sessions\<pid>.json`, and the newest file wins when several sessions are live. `status` is the only field that decides the lamp.

| `status` | Reading |
| --- | --- |
| `busy`, `running` | Working |
| `needs_approval`, `waiting`, `blocked` | Needs confirmation |
| `idle`, `ended`, `stopped` | Idle |

The last segment of `cwd` becomes the detail line. A status Fanal does not recognise, or a file that is not valid JSON, yields no reading at all.

### Codex

`%USERPROFILE%\.codex\sessions\**\rollout-*.jsonl`. The file grows without bound, so only the last 256 KB is read and the partial first line is dropped.

`session_meta` carries the working directory. An `event_msg` whose payload type is `task_started` opens a turn; `task_complete` or `turn_aborted` closes it. An open turn reads as working, a closed one as idle.

Codex records its approval policy once, for the whole thread, rather than per turn. There is no event that reliably says a turn is waiting on the user, so Codex does not produce an amber lamp yet.

### Kimi Code

`%USERPROFILE%\.kimi-code\sessions\**\agents\main\wire.jsonl`, again read from the tail. `agent.turn.started` and `agent.message.appended` read as working; `agent.turn.ended` reads as idle.

## Process names

A process matches on its image name, compared in lowercase with `.exe` removed.

| Agent | Image names |
| --- | --- |
| Claude Code | `claude` |
| Cursor | `cursor` |
| DeepSeek Harness | `deepseek harness` |
| Codex | `codex`, `chatgpt` |
| Kimi | `kimi` |
| Kimi Code | `kimi code`, `kimi-code` |
| Orca | `orca` |
| ChatGPT | `chatgpt` |
| Gemini | `gemini` |
| Trae | `trae`, `trae cn`, `trae solo cn` |
| Kiro | `kiro` |
| Qoder | `qoder` |
| Antigravity | `antigravity` |
| OpenCode | `opencode` |
| Devin | `devin` |

Codex and the ChatGPT desktop app ship under the same image name, so a running ChatGPT app also lights the Codex lamp. Clicking a row focuses that process's window, or launches the executable the running process was started from.

## What is deliberately not read

Session files hold the conversation. The parsers deserialise only the fields named above and drop everything else; nothing from a session is logged, stored, or shown except the short detail line. Fanal keeps its own settings in `%APPDATA%\Fanal\settings.json` and never writes inside an agent's directory.

## Adding an agent

1. Add a variant to `AgentId` in `fanal-core/src/model.rs`, with an explicit serde rename and a display name.
2. Add an `AgentProfile` in `builtin_catalog()` with the image names to match.
3. Add a mark at `ui/marks/<id>.svg` and regenerate `ui/marks.js` with `tools/inline-marks.ps1`.
4. If the agent writes a session file whose format you have verified, add a reader next to the three above and a test that feeds it a real sample. An unverified format should stay process-only.
