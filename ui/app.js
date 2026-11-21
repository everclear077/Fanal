/* Fanal panel. The window is one webview that changes size with its state:
   a thin lip at rest, the agents that need attention when something happens,
   and the full list only while the pointer is over it. */

const invoke = (command, args = {}) =>
  window.__TAURI_INTERNALS__.invoke(command, args);

const MARK_FOR = { "claude-code": "claude" };

const NAMES = {
  "claude-code": "Claude Code",
  cursor: "Cursor",
  "deepseek-harness": "DeepSeek Harness",
  codex: "Codex",
  kimi: "Kimi",
  "kimi-code": "Kimi Code",
  orca: "Orca",
  chatgpt: "ChatGPT",
  gemini: "Gemini",
  trae: "Trae",
  kiro: "Kiro",
  qoder: "Qoder",
  antigravity: "Antigravity",
  opencode: "OpenCode",
  devin: "Devin",
};

const COPY = {
  en: {
    settings: "Settings",
    language: "Language",
    appearance: "Appearance",
    dark: "Dark",
    light: "Light",
    system: "Match system",
    en: "English",
    zh: "Chinese",
    notch: "Notch",
    alert: "Open when something needs you",
    notify: "Notify on an amber lamp",
    connected: "Agents",
    done: "Done",
    open: "Open",
    settingsLabel: "Settings",
  },
  zh: {
    settings: "设置",
    language: "语言",
    appearance: "外观",
    dark: "深色",
    light: "浅色",
    system: "跟随系统",
    en: "English",
    zh: "中文",
    notch: "刘海",
    alert: "有情况才弹出来",
    notify: "黄灯时发系统通知",
    connected: "接入的 Agent",
    done: "完成",
    open: "打开",
    settingsLabel: "设置",
  },
};

const panel = document.getElementById("panel");
const list = document.getElementById("list");
const quiet = document.getElementById("quiet");
const grid = document.getElementById("grid");
const themeBox = document.getElementById("theme");
const languageBox = document.getElementById("language");

let board = null;
let hover = false;
const systemLang = navigator.language.toLowerCase().startsWith("zh") ? "zh" : "en";
let lang = systemLang;
let leaveTimer = 0;
// Lamps from the previous reading, and agents that went from running to
// stopped while the panel was closed. Opening the panel acknowledges them.
let seen = null;
const justStopped = new Set();

const t = (key) => COPY[lang][key];

function applyCopy() {
  document.documentElement.lang = lang === "zh" ? "zh-CN" : "en";
  document.querySelectorAll("[data-i18n]").forEach((node) => {
    node.textContent = t(node.dataset.i18n);
  });
  document.getElementById("gear").setAttribute("aria-label", t("settingsLabel"));
}

function resolvedTheme() {
  if (!board) return "dark";
  if (board.settings.theme === "system") {
    return window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
  }
  return board.settings.theme;
}

function resolvedLang() {
  if (!board || !board.settings.language || board.settings.language === "system") {
    return systemLang;
  }
  return board.settings.language;
}

function noteChanges() {
  const next = {};
  board.agents.forEach((agent) => {
    next[agent.id] = agent.lamp;
    if (seen && !hover && seen[agent.id] === "green" && agent.lamp === "red") {
      justStopped.add(agent.id);
    }
    if (agent.lamp !== "red") justStopped.delete(agent.id);
  });
  seen = next;
}

function needsAttention() {
  return (
    board.severity.attention > 0 ||
    board.agents.some((agent) => justStopped.has(agent.id))
  );
}

function view() {
  if (panel.dataset.page === "settings") return "hover";
  if (hover) return "hover";
  if (needsAttention() && board.settings.alert_on_attention) return "alert";
  return "idle";
}

function render() {
  if (!board) return;
  noteChanges();
  lang = resolvedLang();
  applyCopy();
  panel.dataset.theme = resolvedTheme();
  panel.dataset.view = view();
  document.documentElement.dataset.theme = resolvedTheme();

  const attention = board.agents.filter(
    (agent) => agent.lamp === "amber" || justStopped.has(agent.id),
  );
  const running = board.agents.filter((agent) => agent.lamp === "green");
  const stopped = board.agents.filter(
    (agent) => agent.lamp === "red" && !justStopped.has(agent.id),
  );

  list.replaceChildren();
  attention.concat(running).forEach((agent) => list.appendChild(row(agent)));
  quiet.replaceChildren();
  stopped.forEach((agent) => quiet.appendChild(quietRow(agent)));

  fit();
}

function mark(id) {
  const lamp = document.createElement("span");
  lamp.className = "lamp";
  lamp.innerHTML = MARKS[MARK_FOR[id] || id] || "";
  return lamp;
}

function row(agent) {
  const button = document.createElement("button");
  button.type = "button";
  const tone = agent.lamp === "amber" ? "wait" : agent.lamp === "red" ? "stop" : "go";
  button.className = `row ${tone}`;
  const lamp = mark(agent.id);
  lamp.classList.add(agent.lamp === "amber" ? "wait" : agent.lamp === "red" ? "off" : "go");
  const text = document.createElement("span");
  text.className = "text";
  const name = document.createElement("span");
  name.className = "name";
  name.textContent = NAMES[agent.id] || agent.id;
  const sub = document.createElement("span");
  sub.className = "sub";
  sub.textContent = agent.detail || "";
  text.append(name, sub);
  button.append(lamp, text);
  if (agent.lamp === "amber") {
    const open = document.createElement("span");
    open.className = "open";
    open.textContent = t("open");
    button.append(open);
  }
  button.addEventListener("click", () => invoke("open_agent", { id: agent.id }));
  return button;
}

function quietRow(agent) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "q";
  const lamp = mark(agent.id);
  lamp.classList.add("off");
  const name = document.createElement("span");
  name.textContent = NAMES[agent.id] || agent.id;
  button.append(lamp, name);
  button.addEventListener("click", () => invoke("open_agent", { id: agent.id }));
  return button;
}

function panelWidth() {
  const current = panel.dataset.view;
  if (current === "hover") return 400;
  if (current === "alert") return Math.ceil(panel.scrollWidth);
  return 132;
}

// The window is resized from the content, so the measurement has to happen
// after the browser has laid the new rows out. Measuring inside the same
// turn that built them reports the previous frame's height.
let fitFrame = 0;
function fit() {
  const frame = ++fitFrame;
  requestAnimationFrame(() => {
    if (frame !== fitFrame) return;
    const height = Math.ceil(panel.getBoundingClientRect().height);
    invoke("fit_panel", { width: panelWidth(), height: Math.max(height, 11) });
  });
}

function segment(box, values, current, apply) {
  box.replaceChildren();
  values.forEach((value) => {
    const button = document.createElement("button");
    button.type = "button";
    button.textContent = t(value);
    button.className = current === value ? "on" : "";
    button.addEventListener("click", () => {
      apply(value);
      renderSettings();
      render();
    });
    box.appendChild(button);
  });
}

function renderSettings() {
  segment(languageBox, ["en", "zh", "system"], board.settings.language || "system", (value) => {
    board.settings.language = value;
  });
  segment(themeBox, ["dark", "light", "system"], board.settings.theme, (value) => {
    board.settings.theme = value;
  });
  document.getElementById("alert").checked = board.settings.alert_on_attention;
  document.getElementById("notify").checked = board.settings.notify_on_amber;

  grid.replaceChildren();
  board.settings.agents.forEach((agent) => {
    const label = document.createElement("label");
    label.className = "check";
    const input = document.createElement("input");
    input.type = "checkbox";
    input.checked = agent.enabled;
    input.addEventListener("change", () => {
      agent.enabled = input.checked;
    });
    const name = document.createElement("span");
    name.textContent = NAMES[agent.id] || agent.id;
    label.append(input, name);
    grid.appendChild(label);
  });
}

async function refresh() {
  try {
    board = await invoke("snapshot");
    render();
  } catch (error) {
    console.error(error);
  }
}

document.getElementById("gear").addEventListener("click", () => {
  panel.dataset.page = "settings";
  renderSettings();
  render();
});

document.getElementById("settings").addEventListener("submit", async (event) => {
  event.preventDefault();
  board.settings.alert_on_attention = document.getElementById("alert").checked;
  board.settings.notify_on_amber = document.getElementById("notify").checked;
  await invoke("save_settings", { settings: board.settings });
  panel.dataset.page = "agents";
  await refresh();
});

panel.addEventListener("pointerenter", () => {
  clearTimeout(leaveTimer);
  hover = true;
  justStopped.clear();
  render();
});
panel.addEventListener("pointerleave", () => {
  clearTimeout(leaveTimer);
  leaveTimer = setTimeout(() => {
    hover = false;
    if (panel.dataset.page !== "settings") render();
  }, 400);
});

window.matchMedia("(prefers-color-scheme: light)").addEventListener("change", render);

applyCopy();
refresh();
setInterval(refresh, 2000);
