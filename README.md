# Fanal

Fanal watches the coding agents on this machine and lines them up on a notch at the top of the screen.

Each agent has its own window. A task can be running, sitting on a confirmation, or the program can be closed. To find that out you would open the window. The notch shows it on one strip.

There is no status line under the name. Grey means the program is not running. Yellow means the client is open and no task is running. Green means a task is running. Red means a task has stopped on a confirmation and needs you. Red comes before green, green before yellow, yellow before grey.

[中文说明](README.zh-CN.md)

At rest it is a short black lip. The window is only that tall, so the browser tabs under it stay clickable. It opens by itself when an agent needs you, or when a task that was running has stopped, and it opens all the way while the pointer is over it. Move the pointer away, or click somewhere else, and it closes. Click a row and that agent's window comes to the front, maximised. Fanal does not answer the prompt.

![The panel open, with Codex waiting and Claude Code working](docs/images/panel.png)

The notch at rest, and the same notch after it opened itself because Codex is waiting on `auth.ts`:

![The notch at rest](docs/images/notch.png)
![The notch opened on its own](docs/images/alert.png)

## Settings

The gear at the bottom of the panel opens settings: which agents are watched, whether the notch opens by itself, and the language and theme. Settings are kept in `%APPDATA%\Fanal\settings.json`, which is Fanal's own directory and never an agent's.

![Settings](docs/images/settings.png)

A lamp in the tray follows the most urgent colour on the board. Red outranks green, green outranks yellow, yellow outranks grey.

## What it watches

Claude Code, Claude Desktop, Cursor, DeepSeek Harness, Codex, Kimi, Kimi Code, Gemini, Trae, Kiro, Qoder, Antigravity, OpenCode, and Devin. The Codex app and the ChatGPT desktop app are the same program, so they share one lamp.

Every agent is detected from its process. Claude Code, Claude Desktop, Codex, Kimi, and Kimi Code also have a session file whose format was checked against real files, so those five can say what they are doing rather than only that they are running. The details, including what Fanal refuses to read out of a session file, are in [docs/AGENTS.md](docs/AGENTS.md).

## Build

Fanal is a [Tauri](https://tauri.app) 2 application. The window and the panel are Rust plus a small page with no build step; the decision about which colour a lamp gets is a separate crate, `fanal-core`, with no dependencies beyond serde, so it can be tested on its own.

```
cargo test --manifest-path fanal-core/Cargo.toml
cargo build --manifest-path src-tauri/Cargo.toml
```

The debug build is `src-tauri/target/debug/fanal.exe`. An installer is produced with `cargo tauri build` from `src-tauri`, which needs the [Tauri prerequisites](https://tauri.app/start/prerequisites/) and WebView2.

Rust 1.90 or newer. The panel is Windows only; `fanal-core` is not.

## Layout

```
fanal-core/     lamp decisions and session parsers, with their tests
src-tauri/      the desktop app: process scan, window focus, tray
ui/             the panel: one page, one stylesheet, one script
docs/           how detection works, and the screenshots above
AGENTS.md       rules for working on this repo
CHANGELOG.md    what a person can do after each release
```

[AGENTS.md](AGENTS.md) is the working agreement. [docs/AGENTS.md](docs/AGENTS.md) is a different file: it describes how a lamp is read.

## License

[MIT](LICENSE)
