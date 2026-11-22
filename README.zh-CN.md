# Fanal

Fanal 看着这台电脑上的编程 Agent，把它们排在屏幕顶上的一条刘海里。

每个 Agent 有自己的窗口。任务可能正在跑，可能停在确认框上，也可能程序没开。不看刘海，就得把窗口一个个打开才知道。刘海把这些状态放在一条上。

名字旁边没有一行状态文字。灰是程序没开。黄是客户端开着，没有任务在跑。绿是有任务在跑。红是任务停在确认上，在等你。红灯比绿灯优先，绿灯比黄灯优先，黄灯比灰灯优先。

[English](README.md)

收起来只是一小条黑边，窗口也只有这么高，下面的浏览器标签还能点。某个 Agent 要你看一眼，或者一个正在跑的任务停了，它会自己弹开。鼠标放上去，面板就全部展开。鼠标移开，或者点到别处，它就收回去。点一行，那个 Agent 自己的窗口会到前台，并且最大化。确认框里的问题，Fanal 不替你回答。

![面板展开，Codex 在等待，Claude Code 在工作](docs/images/panel.png)

收起时的刘海，以及 Codex 在 `auth.ts` 上等确认、刘海自己弹开的样子：

![收起的刘海](docs/images/notch.png)
![自己弹开的刘海](docs/images/alert.png)

## 设置

面板底部的齿轮打开设置：监视哪些 Agent、刘海要不要自己弹开，以及语言和主题。设置保存在 `%APPDATA%\Fanal\settings.json`，这是 Fanal 自己的目录，不是任何一个 Agent 的。

![设置](docs/images/settings.png)

托盘里的灯跟着整块面板上最紧急的颜色走。红灯优先于绿灯，绿灯优先于黄灯，黄灯优先于灰灯。

## 它监视哪些

Claude Code、Claude Desktop、Cursor、DeepSeek Harness、Codex、Kimi、Kimi Code、Gemini、Trae、Kiro、Qoder、Antigravity、OpenCode 和 Devin。Codex 和 ChatGPT 桌面版是同一个程序，共用一盏灯。

每个 Agent 都从进程判断。Claude Code、Claude Desktop、Codex、Kimi 和 Kimi Code 另外有一份拿真实文件核过格式的会话文件，所以这五个能说出自己在做什么，而不只是在不在跑。细节，包括 Fanal 拒绝从会话文件里读什么，写在 [docs/agents.zh-CN.md](docs/agents.zh-CN.md)。

## 构建

Fanal 是一个 [Tauri](https://tauri.app) 2 应用。窗口和面板是 Rust 加上一个没有构建步骤的小页面；灯该是什么颜色这个判断单独放在 `fanal-core` 里，除了 serde 没有别的依赖，所以可以单独测试。

```
cargo test --manifest-path fanal-core/Cargo.toml
cargo build --manifest-path src-tauri/Cargo.toml
```

调试版在 `src-tauri/target/debug/fanal.exe`。在 `src-tauri` 里运行 `cargo tauri build` 会打出安装包，需要 [Tauri 的先决条件](https://tauri.app/start/prerequisites/) 和 WebView2。

需要 Rust 1.90 或更新的版本。面板只在 Windows 上运行，`fanal-core` 不限平台。

## 目录

```
fanal-core/     灯的判断和会话解析，以及对应的测试
src-tauri/      桌面应用：进程扫描、窗口聚焦、托盘
ui/             面板：一个页面、一份样式、一段脚本
docs/           检测方式的说明，以及上面这些截图
AGENTS.md       改这个仓库时要遵守的规则
CHANGELOG.md    每个版本里，使用者新能做什么
```

[AGENTS.md](AGENTS.md) 是工作约定。[docs/AGENTS.md](docs/AGENTS.md) 是另一份文件，写的是一盏灯怎么读。

## 许可证

[MIT](LICENSE)
