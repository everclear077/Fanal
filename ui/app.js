/* Fanal panel. The window is one webview that changes size with its state:
   a thin lip at rest, the agents that need attention when something happens,
   and the full list only while the pointer is over it. */

const invoke = (command, args = {}) =>
  window.__TAURI_INTERNALS__.invoke(command, args);

const MARK_FOR = { "claude-code": "claude", "claude-desktop": "claude" };

const NAMES = {
  "claude-code": "Claude Code",
  "claude-desktop": "Claude Desktop",
  cursor: "Cursor",
  "deepseek-harness": "DeepSeek Harness",
  codex: "Codex",
  kimi: "Kimi",
  "kimi-code": "Kimi Code",
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
    // A confirmation is red, not a task that stopped. Only a task that was
    // actually running and then finished opens the notch on its own.
    if (seen && !hover && seen[agent.id] === "green" && (agent.lamp === "yellow" || agent.lamp === "grey")) {
      justStopped.add(agent.id);
    }
    if (agent.lamp === "green" || agent.lamp === "red") justStopped.delete(agent.id);
  });
  seen = next;
}

function needsAttention() {
  return board.agents.some((agent) => agent.lamp === "red" || justStopped.has(agent.id));
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
    (agent) => agent.lamp === "red" || justStopped.has(agent.id),
  );
  const attentionIds = new Set(attention.map((agent) => agent.id));
  // Grey with nothing behind it means the program is not installed. It stays
  // out of the list; the settings page is where it gets switched on.
  const present = board.agents.filter(
    (agent) => agent.lamp !== "grey" || agent.from_session || justStopped.has(agent.id),
  );
  const running = present.filter(
    (agent) =>
      (agent.lamp === "green" || agent.lamp === "yellow") && !attentionIds.has(agent.id),
  );
  const stopped = present.filter(
    (agent) => agent.lamp === "grey" && !justStopped.has(agent.id),
  );

  list.replaceChildren();
  attention.concat(running).forEach((agent) => list.appendChild(row(agent, attentionIds.has(agent.id))));
  quiet.replaceChildren();
  stopped.forEach((agent) => quiet.appendChild(quietRow(agent)));
  document.getElementById("rule").hidden = stopped.length === 0;

  fit();
}

function mark(id) {
  const node = document.createElement("span");
  node.className = "mark";
  node.innerHTML = MARKS[MARK_FOR[id] || id] || "";
  return node;
}

// A solid round button. Green is a task executing, yellow an open client with
// nothing running, red a confirmation waiting on the user, grey a closed one.
function bulb(lamp) {
  const tone = { green: "go", yellow: "idle", red: "need", grey: "off" }[lamp] || "off";
  const node = document.createElement("span");
  node.className = "bulb " + tone;
  return node;
}

function row(agent, popped) {
  const button = document.createElement("button");
  button.type = "button";
  const tone = { green: "go", yellow: "idle", red: "need", grey: "stop" }[agent.lamp] || "stop";
  button.className = popped ? `row ${tone} pop` : `row ${tone}`;
  const text = document.createElement("span");
  text.className = "text";
  const name = document.createElement("span");
  name.className = "name";
  name.textContent = NAMES[agent.id] || agent.id;
  text.append(name);
  const side = document.createElement("span");
  side.className = "side";
  if (agent.detail) {
    const sub = document.createElement("span");
    sub.className = "sub";
    sub.textContent = agent.detail;
    side.append(sub);
  }
  if (agent.lamp === "red") {
    const open = document.createElement("span");
    open.className = "open";
    open.textContent = t("open");
    side.append(open);
  }
  side.append(bulb(agent.lamp));
  button.append(mark(agent.id), text, side);
  button.addEventListener("click", () => invoke("open_agent", { id: agent.id }));
  return button;
}

function quietRow(agent) {
  return row(agent);
}

const HOVER_WIDTH = 332;

function panelWidth() {
  const current = panel.dataset.view;
  if (current === "hover") return HOVER_WIDTH;
  if (current !== "alert") return 132;
  const pops = list.querySelectorAll(".row.pop");
  if (list.querySelector(".row.pop.need")) return Math.min(HOVER_WIDTH, 300);
  return Math.min(HOVER_WIDTH, Math.max(132, 24 + pops.length * 52));
}

// Measure at the width the window is about to become. Measuring at the lip's
// width wraps every row, the page grows a scrollbar, and the window then
// changes size again. The card stays hidden until that resize has landed.
let fitFrame = 0;
function fit() {
  const frame = ++fitFrame;
  const width = panelWidth();
  const entering = panel.dataset.seenView !== panel.dataset.view;
  if (entering) delete panel.dataset.settled;
  requestAnimationFrame(() => {
    if (frame !== fitFrame) return;
    panel.style.width = width + "px";
    const height = Math.max(11, Math.ceil(panel.offsetHeight));
    panel.style.width = "";
    const key = width + "x" + height;
    if (!entering && panel.dataset.fit === key && panel.dataset.settled === "1") return;
    panel.dataset.fit = key;
    invoke("fit_panel", { width, height, note: "" }).then(() => {
      if (frame !== fitFrame) return;
      panel.dataset.seenView = panel.dataset.view;
      panel.dataset.settled = "1";
    });
  });
}

function segment(box, values, current, apply) {
  box.replaceChildren();
  values.forEach((value) => {
    const button = document.createElement("button");
    button.type = "button";
    button.dataset.value = value;
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

  grid.replaceChildren();
  board.settings.agents.forEach((agent) => {
    const label = document.createElement("label");
    label.className = "check";
    const input = document.createElement("input");
    input.type = "checkbox";
    input.dataset.id = agent.id;
    input.checked = agent.enabled;
    const name = document.createElement("span");
    name.textContent = NAMES[agent.id] || agent.id;
    label.append(input, name);
    grid.appendChild(label);
  });
}

// The form is the copy being edited. A poll must not replace it, or the
// click on Done saves the reading from before the boxes were changed.
let poll = 0;
async function refresh() {
  const ticket = ++poll;
  try {
    const next = await invoke("snapshot");
    if (ticket !== poll || !next) return;
    if (panel.dataset.page === "settings" && board) next.settings = board.settings;
    board = next;
    render();
  } catch (error) {
    console.error(error);
  }
}

function chosen(box) {
  const button = box.querySelector("button.on");
  return button ? button.dataset.value : "";
}

function applyForm() {
  board.settings.alert_on_attention = document.getElementById("alert").checked;
  const language = chosen(languageBox);
  const theme = chosen(themeBox);
  if (language) board.settings.language = language;
  if (theme) board.settings.theme = theme;
  grid.querySelectorAll("input[data-id]").forEach((input) => {
    const agent = board.settings.agents.find((item) => item.id === input.dataset.id);
    if (agent) agent.enabled = input.checked;
  });
}

document.getElementById("gear").addEventListener("click", () => {
  panel.dataset.page = "settings";
  renderSettings();
  render();
});

document.getElementById("settings").addEventListener("submit", async (event) => {
  event.preventDefault();
  applyForm();
  poll += 1;
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
function collapse() {
  clearTimeout(leaveTimer);
  hover = false;
  const discard = panel.dataset.page === "settings";
  panel.dataset.page = "agents";
  if (discard) refresh();
  else render();
}

panel.addEventListener("pointerleave", () => {
  clearTimeout(leaveTimer);
  // The window is exactly as tall as the panel, so the moment the pointer
  // leaves it is already over whatever sits underneath. Waiting here keeps
  // that window on top of the browser tab the pointer just reached.
  collapse();
});

// Clicking another window does not move the pointer out of this one when the
// cursor was resting on it, so focus loss closes it too.
window.__fanalFocus = (focused) => {
  if (focused) {
    clearTimeout(leaveTimer);
    return;
  }
  clearTimeout(leaveTimer);
  leaveTimer = setTimeout(() => {
    if (!document.hasFocus()) collapse();
  }, 80);
};

window.matchMedia("(prefers-color-scheme: light)").addEventListener("change", render);

applyCopy();
refresh();
setInterval(refresh, 2000);
