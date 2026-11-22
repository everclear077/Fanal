# Fanal — Project Rules

> Extends ~/CLAUDE.md. Project rules override global when conflict.
> A notch on Windows: one lamp per local coding agent.
> `docs/AGENTS.md` explains a lamp. This file explains how to change the repo.

## What this is

Fanal is a Tauri 2 window plus the pure `fanal-core` crate. Grey means the program is closed, yellow means the client is open and idle, green means a task is executing, and red means a task opened a confirmation. Red outranks green, then yellow, then grey. A closed program is never red.

## Session Checklist

1. Lamp behaviour → `fanal-core` and `docs/AGENTS.md`. The Chinese twin is `docs/agents.zh-CN.md`.
2. Process scan, focus, or the tray → `src-tauri`. Read `src-tauri/CLAUDE.md` first.
3. Panel layout → `ui`. Read `ui/CLAUDE.md` first.
4. Branch state → `HANDOFF.md` when it exists. It stays on this machine.

## Workflow

- Stay on `main` unless the user asks for another branch.
- Commit only when the user asks. One commit is one logical change, and the subject is a sentence.
- Before a commit that changes behaviour, run `cargo test --manifest-path fanal-core/Cargo.toml` and `cargo test --manifest-path src-tauri/Cargo.toml --offline --lib`.
- Push, tag, and release only when the user asks. Ask before `git push --force`, `git reset --hard`, or a bulk delete.
- Run git inside `Fanal`. The parent directory is not this repository.

## Truth Hierarchy

1. A lamp meaning the user stated in the current conversation.
2. The tests in `fanal-core`, then the code.
3. `docs/AGENTS.md` and `docs/agents.zh-CN.md`. They must say the same thing.
4. The READMEs, then this file. This file never overrides a lamp meaning.

## Session Protocol

Before ending a session:

1. Update the `main` section of `HANDOFF.md`. Do not commit that file.
2. Add a `CHANGELOG.md` line starting "You can now" when behaviour a person can see has changed.
3. Do not commit session files, Fanal settings, screenshots outside `docs/images`, or a personal path.

## Where things live

- `fanal-core/` judges the lamp and parses session status. It has no window code.
- `src-tauri/` scans processes, focuses windows, and owns the tray.
- `ui/` is the panel. There is no bundler: `frontendDist` is `../ui`, so a UI change needs `cargo build`.
- `docs/` is how detection works. `tools/` regenerates the icons and captures the panel.
