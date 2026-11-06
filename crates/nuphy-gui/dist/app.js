"use strict";

const $ = s => document.querySelector(s);
const t = (k, v) => i18n.t(k, v);

function h(tag, attrs, ...kids) {
  const el = document.createElement(tag);
  for (const k in attrs || {}) {
    const v = attrs[k];
    if (k === "class") el.className = v;
    else if (k.startsWith("on")) el.addEventListener(k.slice(2), v);
    else if (v === true) el.setAttribute(k, "");
    else if (v !== false && v != null) el.setAttribute(k, v);
  }
  for (const kid of kids.flat()) if (kid != null && kid !== false) el.append(kid);
  return el;
}

const PRESETS = ["esc", "caps", "pscr", "del", "ins", "home", "end", "pgup", "pgdn", "lgui", "mute", "vold", "volu", "mply", "trns", "no"];
const UDEV_CMD = "sudo cp contrib/udev/70-nuphy.rules /etc/udev/rules.d/ && sudo udevadm control --reload && sudo udevadm trigger --subsystem-match=hidraw";

const S = { st: null, layer: 0, sel: null, busy: false, error: null, draft: "", previewSeq: 0 };

// ---------- helpers ----------
function errInfo(e) {
  if (e && typeof e === "object" && e.code) return e;
  return { code: "other", detail: String(e && e.message || e) };
}
function errText(e) {
  const { code, detail } = errInfo(e);
  const base = t("err." + code);
  const showDetail = ["other", "bad_key", "verify_failed", "restore_failed"].includes(code) && detail;
  return showDetail ? base + " (" + detail + ")" : base;
}
let toastTimer = 0;
function toast(msg, isError) {
  const el = $("#toast");
  el.textContent = msg;
  el.className = "toast" + (isError ? " error" : "");
  el.hidden = false;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => { el.hidden = true; }, isError ? 6000 : 3200);
}
const slotOf = (layer, slot) => layer * S.st.slots_per_layer + slot;
const keyAt = (layer, slot) => S.st.keys[slotOf(layer, slot)];
const defAt = (layer, slot) => S.st.defaults[slotOf(layer, slot)];
function posInfo(slot) {
  const m = S.st.rows * S.st.cols;
  return slot < m ? { row: Math.floor(slot / S.st.cols), col: slot % S.st.cols } : { extra: slot - m };
}
function keyLabel(code, label) {
  if (code === 0) return "";
  if (code === 1) return "▽";
  return label;
}

// ---------- connection ----------
async function connect() {
  if (S.busy) return;
  S.busy = true; S.error = null; render();
  try {
    const st = await api.call("connect");
    S.st = st;
    S.layer = st.active_layer;
    S.sel = null;
  } catch (e) {
    S.st = null; S.error = errInfo(e);
  }
  S.busy = false; render();
}

async function reload() {
  if (!S.st) return connect();
  try {
    S.st = await api.call("refresh");
    render();
  } catch (e) { S.st = null; S.error = errInfo(e); render(); }
}

// ---------- rendering ----------
function render() {
  const main = $("#main");
  main.replaceChildren();
  renderDevice();
  if (api.demo) main.append(h("div", { class: "banner" }, t("demo.banner")));
  if (!S.st) { main.append(renderConnect()); return; }
  main.append(h("div", { class: "workspace" },
    h("section", { class: "board" }, renderLayers(), renderGrid(), h("p", { class: "legend muted" }, t("grid.legend"))),
    h("section", { class: "editor card", id: "editor" })));
  renderEditor();
}

function renderDevice() {
  const d = $("#device");
  d.replaceChildren();
  if (!S.st) return;
  d.append(
    h("span", { class: "dot ok" }),
    h("strong", null, S.st.model || S.st.name),
    h("span", { class: "muted" }, `${t("dev.fw")} ${S.st.firmware} · ${t("dev.mode")} ${S.st.current_mode} · ` +
      `${S.st.vid.toString(16)}:${S.st.pid.toString(16)}`));
}

function renderConnect() {
  const e = S.error;
  const kids = [h("h2", null, t("conn.title")), h("p", { class: "muted" }, t("conn.hint"))];
  if (e) {
    kids.push(h("p", { class: "error-text" }, errText(e)));
    if (e.code === "permission") {
      kids.push(h("pre", { class: "cmd" }, UDEV_CMD));
      kids.push(h("button", { class: "ghost", onclick: () => navigator.clipboard && navigator.clipboard.writeText(UDEV_CMD) }, "Copy"));
    }
  }
  kids.push(h("button", { class: "primary", disabled: S.busy, onclick: connect }, S.busy ? t("conn.connecting") : t("conn.connect")));
  return h("div", { class: "card connect" }, kids);
}

function renderLayers() {
  const st = S.st;
  const modes = Math.ceil(st.layer_count / st.layers_per_mode);
  const groups = [];
  for (let m = 0; m < modes; m++) {
    const tabs = [];
    for (let i = 0; i < st.layers_per_mode; i++) {
      const l = m * st.layers_per_mode + i;
      if (l >= st.layer_count) break;
      tabs.push(h("button", { class: "tab" + (l === S.layer ? " on" : ""), onclick: () => { S.layer = l; S.sel = null; render(); } }, String(l)));
    }
    groups.push(h("div", { class: "mode" + (m === st.current_mode ? " current" : "") },
      h("span", { class: "mode-name" }, t("layers.mode", { n: m }), m === st.current_mode ? h("em", null, t("layers.active")) : null),
      h("div", { class: "tabs" }, tabs)));
  }
  return h("div", { class: "layers" }, h("span", { class: "muted" }, t("layers.title")), groups);
}

function cell(slot) {
  const k = keyAt(S.layer, slot);
  const changed = k.code !== defAt(S.layer, slot);
  const cls = ["key"];
  if (k.code === 0) cls.push("none");
  if (k.code === 1) cls.push("trns");
  if (changed) cls.push("changed");
  if (S.sel === slot) cls.push("sel");
  return h("button", { class: cls.join(" "), title: k.label + "  0x" + k.code.toString(16).padStart(4, "0"), onclick: () => select(slot) },
    keyLabel(k.code, k.label));
}

function renderGrid() {
  const { rows, cols } = S.st;
  const grid = h("div", { class: "grid", style: `--cols:${cols}` });
  for (let r = 0; r < rows; r++) for (let c = 0; c < cols; c++) grid.append(cell(r * cols + c));
  const kids = [grid];
  if (S.st.extra > 0) {
    const ex = h("div", { class: "grid extra", style: `--cols:${S.st.extra}` });
    for (let i = 0; i < S.st.extra; i++) ex.append(cell(rows * cols + i));
    kids.push(h("div", { class: "extra-wrap" }, h("span", { class: "muted" }, t("grid.extra")), ex));
  }
  return h("div", { class: "gridwrap" }, kids);
}

function select(slot) {
  S.sel = slot;
  S.draft = "";
  document.querySelectorAll(".key.sel").forEach(e => e.classList.remove("sel"));
  render();
}

// ---------- editor ----------
function renderEditor() {
  const box = $("#editor");
  if (!box) return;
  box.replaceChildren();
  box.append(h("h2", null, t("key.title")));
  if (S.sel == null) { box.append(h("p", { class: "muted" }, t("key.none"))); return; }

  const slot = S.sel, layer = S.layer;
  const cur = keyAt(layer, slot), defCode = defAt(layer, slot);
  const defLabel = (S.st.keys.length && labelOfDefault(layer, slot));
  const p = posInfo(slot);
  box.append(
    h("p", { class: "pos" }, p.extra != null ? t("key.posExtra", { layer, n: p.extra }) : t("key.pos", { layer, row: p.row, col: p.col })),
    h("dl", { class: "kv" },
      h("dt", null, t("key.current")), h("dd", null, h("span", { class: "chip big" }, cur.label), h("code", null, hex(cur.code))),
      h("dt", null, t("key.default")), h("dd", null, h("span", { class: "chip" }, defLabel), h("code", null, hex(defCode)))));

  const input = h("input", { id: "key-input", type: "text", spellcheck: "false", autocomplete: "off", placeholder: t("key.placeholder"), value: S.draft });
  const preview = h("div", { id: "preview", class: "preview muted" });
  const apply = h("button", { class: "primary", id: "btn-apply", disabled: true }, t("common.apply"));
  const reset = h("button", { class: "ghost", disabled: cur.code === defCode, onclick: () => doReset(layer, slot) }, t("key.reset"));
  input.addEventListener("input", () => { S.draft = input.value; updatePreview(preview, apply, cur.code); });
  input.addEventListener("keydown", ev => { if (ev.key === "Enter" && !apply.disabled) apply.click(); });
  apply.addEventListener("click", () => doApply(layer, slot, S.draft));

  box.append(
    h("label", { for: "key-input", class: "lbl" }, t("key.new")), input, preview,
    h("p", { class: "muted small" }, t("key.help")),
    h("div", { class: "row" }, apply, reset),
    h("span", { class: "lbl" }, t("key.quick")),
    h("div", { class: "presets" }, PRESETS.map(n =>
      h("button", { class: "chip btn", onclick: () => { S.draft = n; input.value = n; updatePreview(preview, apply, cur.code); } }, t("preset." + n)))));
  if (S.draft) updatePreview(preview, apply, cur.code);
}

function labelOfDefault(layer, slot) {
  // The backend sends default codes only; ask for the label through the same parser the editor uses.
  const code = defAt(layer, slot);
  return code === 0 ? "NO" : code === 1 ? "TRNS" : (S.defLabels[code] || "0x" + code.toString(16).padStart(4, "0"));
}
S.defLabels = {};
async function warmDefaultLabels() {
  const codes = [...new Set(S.st.defaults)];
  await Promise.all(codes.filter(c => c > 1 && !(c in S.defLabels)).map(async c => {
    try { S.defLabels[c] = (await api.call("parse_key", { text: "0x" + c.toString(16) })).label; } catch (_) {}
  }));
}
const hex = c => "0x" + c.toString(16).padStart(4, "0");

async function updatePreview(preview, apply, curCode) {
  const text = S.draft.trim();
  const seq = ++S.previewSeq;
  if (!text) { preview.className = "preview muted"; preview.textContent = ""; apply.disabled = true; return; }
  try {
    const k = await api.call("parse_key", { text });
    if (seq !== S.previewSeq) return;
    const same = k.code === curCode;
    preview.className = "preview ok";
    preview.textContent = `→ ${k.label}  (${hex(k.code)})` + (same ? "  · " + t("key.same") : "");
    apply.disabled = same;
  } catch (e) {
    if (seq !== S.previewSeq) return;
    preview.className = "preview bad";
    preview.textContent = errText(e);
    apply.disabled = true;
  }
}

async function doApply(layer, slot, text) {
  const before = keyAt(layer, slot).label;
  try {
    const k = await api.call("set_key", { layer, slot, text });
    S.st.keys[slotOf(layer, slot)] = k;
    S.draft = "";
    toast(t("toast.saved", { from: before, to: k.label }));
    render();
  } catch (e) { toast(errText(e), true); }
}

async function doReset(layer, slot) {
  try {
    const k = await api.call("reset_key", { layer, slot });
    S.st.keys[slotOf(layer, slot)] = k;
    S.draft = "";
    toast(t("toast.reset", { to: k.label }));
    render();
  } catch (e) { toast(errText(e), true); }
}

// ---------- backups drawer ----------
async function renderBackups() {
  const ul = $("#backup-list");
  ul.replaceChildren();
  let list = [];
  try { list = await api.call("list_backups"); } catch (e) { toast(errText(e), true); }
  if (!list.length) { ul.append(h("li", { class: "muted" }, t("backups.empty"))); return; }
  const loc = i18n.lang === "zh" ? "zh-CN" : "en-US";
  for (const b of list.slice(0, 50)) {
    ul.append(h("li", null,
      h("span", null, new Date(b.time_ms).toLocaleString(loc)),
      h("button", { class: "ghost", disabled: !S.st, onclick: () => restore(b) }, t("backups.restore"))));
  }
}
async function restore(b) {
  if (!confirm(t("backups.confirm"))) return;
  try {
    S.st = await api.call("restore_backup", { path: b.path });
    toast(t("toast.restored"));
    render(); renderBackups();
  } catch (e) { toast(errText(e), true); }
}

// ---------- wiring ----------
$("#btn-lang").addEventListener("click", () => i18n.toggle());
$("#btn-refresh").addEventListener("click", reload);
$("#btn-backups").addEventListener("click", () => { $("#backups").hidden = false; renderBackups(); });
$("#btn-backups-close").addEventListener("click", () => { $("#backups").hidden = true; });
$("#btn-make-backup").addEventListener("click", async () => {
  try { await api.call("make_backup"); toast(t("toast.backup")); renderBackups(); } catch (e) { toast(errText(e), true); }
});
window.addEventListener("langchange", () => { render(); if (!$("#backups").hidden) renderBackups(); });

i18n.init();
connect().then(() => { if (S.st) warmDefaultLabels().then(render); });
