"use strict";

const $ = s => document.querySelector(s);
const t = (k, v) => i18n.t(k, v);

function h(tag, attrs, ...kids) {
  const el = document.createElement(tag);
  for (const k in attrs || {}) {
    const v = attrs[k];
    if (k === "class") el.className = v;
    else if (k === "style" && typeof v === "object") for (const p in v) el.style.setProperty(p, v[p]);
    else if (k.startsWith("on")) el.addEventListener(k.slice(2), v);
    else if (v === true) el.setAttribute(k, "");
    else if (v !== false && v != null) el.setAttribute(k, v);
  }
  for (const kid of kids.flat()) if (kid != null && kid !== false) el.append(kid);
  return el;
}

const UDEV_CMD = "sudo cp contrib/udev/70-nuphy.rules /etc/udev/rules.d/ && sudo udevadm control --reload && sudo udevadm trigger --subsystem-match=hidraw";
const CATS = [
  { id: "basic", icon: "A" }, { id: "media", icon: "♪" }, { id: "light", icon: "☀" },
  { id: "special", icon: "◇" }, { id: "custom", icon: "#" }
];
const TABS = ["keys", "advanced", "light", "macro", "mode"];

const S = {
  st: null, mode: 0, layer: 0, sel: null, busy: false, connecting: false, error: null,
  cat: "basic", query: "", draft: "", previewSeq: 0
};

// ---------- helpers ----------
function errInfo(e) {
  if (e && typeof e === "object" && e.code) return e;
  return { code: "other", detail: String(e && e.message || e) };
}
function errText(e) {
  const { code, detail } = errInfo(e);
  const base = t("err." + code);
  return ["other", "bad_key", "verify_failed", "restore_failed"].includes(code) && detail ? base + " (" + detail + ")" : base;
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
const per = () => S.st.slots_per_layer;
const keyAt = (layer, slot) => S.st.keys[layer * per() + slot];
const defAt = (layer, slot) => S.st.defaults[layer * per() + slot];
const modeCount = () => Math.ceil(S.st.layer_count / S.st.layers_per_mode);
const modeName = m => modeCount() === 2 ? ["Mac", "Windows"][m] : "";
const isMac = () => modeCount() === 2 && S.mode === 0;
const capOf = code => Keys.cap(code, isMac());
const textOf = (code, label) => Keys.text(code, isMac(), label);
function posText(slot) {
  const m = S.st.rows * S.st.cols;
  return slot < m ? t("key.pos", { row: Math.floor(slot / S.st.cols), col: slot % S.st.cols })
    : t("key.posExtra", { n: slot - m + 1 });
}
function icon(d) {
  const span = document.createElement("span");
  span.innerHTML = `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="${d}"/></svg>`;
  return span.firstChild;
}
/** Font scale for a keycap label, so long words still fit. */
function capScale(c) {
  const lines = Array.isArray(c) ? c : String(c).split("\n");
  const n = Math.max(...lines.map(l => [...l].length));
  return n >= 9 ? 0.62 : n >= 7 ? 0.72 : n >= 5 ? 0.86 : 1;
}
function capContent(c) {
  if (Array.isArray(c)) return [h("span", null, c[0]), h("span", null, c[1])];
  return String(c).split("\n").map(line => h("span", null, line));
}

// ---------- connection ----------
async function connect() {
  if (S.connecting) return;
  S.connecting = true; S.error = null; render();
  try {
    adopt(await api.call("connect"));
    S.mode = S.st.current_mode;
    S.layer = S.st.active_layer;
    S.sel = null;
  } catch (e) {
    S.st = null; S.error = errInfo(e);
  }
  S.connecting = false; render();
}
function adopt(st) {
  S.st = st;
  if (S.layer >= st.layer_count) S.layer = st.active_layer;
}
async function reload() {
  if (!S.st) return connect();
  try { adopt(await api.call("refresh")); render(); toast(t("toast.reloaded")); }
  catch (e) { S.st = null; S.error = errInfo(e); render(); }
}

// ---------- rendering ----------
function render() {
  renderHeader();
  const main = $("#main");
  main.replaceChildren();
  if (api.demo) main.append(h("div", { class: "banner" }, t("demo.banner")));
  if (!S.st) { main.append(renderConnect()); return; }
  main.append(
    h("section", { class: "stage", id: "stage" }),
    h("section", { class: "panel-wrap" },
      h("div", { class: "tabs", role: "tablist" }, TABS.map((id, i) =>
        h("button", { class: i === 0 ? "on" : "", role: "tab", "aria-selected": String(i === 0), disabled: i !== 0,
          title: i === 0 ? null : t("tab.soon") }, t("tab." + id)))),
      h("div", { class: "panel first-tab" }, h("aside", { class: "cats", id: "cats" }), h("div", { class: "pal", id: "pal" }))));
  renderStage(); renderCats(); renderPal();
  fit();
}

function renderHeader() {
  const d = $("#device");
  d.replaceChildren();
  const wrap = $("#mode-wrap");
  wrap.hidden = !S.st;
  if (!S.st) return;
  d.append(h("span", { class: "dot" }), h("strong", null, S.st.name || S.st.model),
    h("span", { class: "meta" }, `${t("dev.fw")} ${S.st.firmware} · ${S.st.vid.toString(16)}:${S.st.pid.toString(16)}`));
  const sel = $("#mode-select");
  sel.replaceChildren();
  for (let m = 0; m < modeCount(); m++) {
    const name = [`M${m + 1}`, modeName(m)].filter(Boolean).join(" ");
    sel.append(h("option", { value: String(m) }, m === S.st.current_mode ? `${name} · ${t("mode.inUse")}` : name));
  }
  sel.value = String(S.mode);
}

function renderConnect() {
  const e = S.error;
  const kids = [h("h2", null, t("conn.title")), h("p", { class: "muted" }, t("conn.hint"))];
  if (e) {
    kids.push(h("p", { class: "error-text" }, errText(e)));
    if (e.code === "permission") {
      kids.push(h("pre", { class: "cmd" }, UDEV_CMD));
      const copy = h("button", { class: "btn" }, t("common.copy"));
      copy.addEventListener("click", async () => {
        try { await navigator.clipboard.writeText(UDEV_CMD); copy.textContent = t("common.copied"); } catch (_) {}
      });
      kids.push(copy);
    }
  }
  kids.push(h("button", { class: "btn dark", disabled: S.connecting, onclick: connect },
    S.connecting ? t("conn.connecting") : t("conn.connect")));
  return h("div", { class: "connect" }, kids);
}

function cap(slot, extraStrip) {
  const k = keyAt(S.layer, slot);
  const cls = ["cap"];
  if (k.code === 0) cls.push("none");
  if (k.code === 1) cls.push("trns");
  if (k.code !== defAt(S.layer, slot)) cls.push("changed");
  if (S.sel === slot) cls.push("sel");
  const title = `${posText(slot)} · ${textOf(k.code, k.label)} (${Keys.hex(k.code)})`;
  const label = capOf(k.code) || " ";
  const style = { "--fs": capScale(label) };
  if (extraStrip) Object.assign(style, { "--w": 1.25, "--h": 1 });
  return h("button", { class: cls.join(" "), title, "aria-label": title, "aria-pressed": String(S.sel === slot),
    "data-slot": slot, style, onclick: () => select(slot) },
    h("span", { class: "out" }, h("span", { class: "in" }, capContent(label))));
}

function renderStage() {
  const stage = $("#stage");
  if (!stage) return;
  const lay = Layout.forState(S.st);
  const plate = h("div", { class: "kb-plate", style: { width: `calc(${lay.w} * var(--u))`, height: `calc(${lay.h} * var(--u))` } });
  for (const k of lay.keys) {
    const el = cap(k.slot);
    el.style.setProperty("--x", k.x); el.style.setProperty("--y", k.y);
    el.style.setProperty("--w", k.w); el.style.setProperty("--h", k.h);
    if (k.w >= 1.5) el.style.setProperty("--fs", Math.max(+el.style.getPropertyValue("--fs"), k.w >= 2 ? 1 : 0.86));
    plate.append(el);
  }
  const kbCase = h("div", { class: "kb-case" + (lay.real ? "" : " grid-mode"), id: "kb-case" }, plate);

  const kids = [h("div", { class: "board-wrap", id: "board-wrap" }, kbCase)];
  if (!lay.real) kids.push(h("p", { class: "muted small" }, t("board.gridNote")));
  if (lay.extras.length) {
    kids.push(h("div", { class: "extras" }, h("span", null, t("board.extra")),
      h("div", { class: "caps", id: "extra-caps" }, lay.extras.map(s => cap(s, true)))));
  }
  kids.push(renderBar());
  stage.replaceChildren(...kids);
}

function renderBar() {
  const info = h("div", { class: "key-info" });
  const canReset = S.sel != null && keyAt(S.layer, S.sel).code !== defAt(S.layer, S.sel);
  if (S.sel == null) {
    info.append(h("span", { class: "muted" }, t("board.hint")));
  } else {
    const cur = keyAt(S.layer, S.sel), def = defAt(S.layer, S.sel);
    info.append(
      h("span", { class: "pos" }, posText(S.sel)),
      h("span", { class: "chip" }, t("key.current"), h("b", null, textOf(cur.code, cur.label)), h("code", null, Keys.hex(cur.code))),
      h("span", { class: "chip" }, t("key.default"), h("b", null, textOf(def)), h("code", null, Keys.hex(def))));
  }
  const reset = h("button", { class: "btn plain", disabled: !canReset || S.busy, title: t("key.resetTip"), onclick: doReset },
    icon("M12 5V2L7 6l5 4V7a5 5 0 1 1-5 5H5a7 7 0 1 0 7-7Z"), t("key.reset"));
  const seg = h("div", { class: "segment", role: "group", "aria-label": t("layer.label") });
  const first = S.mode * S.st.layers_per_mode;
  for (let l = first; l < Math.min(first + S.st.layers_per_mode, S.st.layer_count); l++) {
    seg.append(h("button", { class: l === S.layer ? "on" : "", "aria-pressed": String(l === S.layer),
      title: l === first ? t("layer.base", { n: l }) : t("layer.fn", { n: l }),
      onclick: () => { S.layer = l; refreshViews(); } }, `FN ${l}`));
  }
  return h("div", { class: "board-bar" }, info, h("div", { class: "bar-right" }, reset, seg));
}

function renderCats() {
  const box = $("#cats");
  if (!box) return;
  const list = h("div", { class: "list", role: "tablist" }, CATS.map(c =>
    h("button", { class: S.cat === c.id && !S.query ? "on" : "", role: "tab", onclick: () => { S.cat = c.id; S.query = ""; renderCats(); renderPal(); fit(); } },
      h("span", { class: "ico", "aria-hidden": "true" }, c.icon), t("cat." + c.id))));
  const input = h("input", { type: "search", value: S.query, placeholder: t("search.placeholder"), "aria-label": t("search.placeholder") });
  input.addEventListener("input", () => {
    S.query = input.value.trim();
    list.querySelectorAll("button").forEach((b, i) => b.classList.toggle("on", !S.query && CATS[i].id === S.cat));
    renderPal(); fit();
  });
  box.replaceChildren(list, h("label", { class: "search" },
    h("span", { "aria-hidden": "true" }, "⌕"), input));
}

function pk(code, geo) {
  const cur = S.sel != null && keyAt(S.layer, S.sel).code === code;
  const label = capOf(code) || " ";
  const el = h("button", { class: "pk" + (cur ? " cur" : ""), title: `${textOf(code)} (${Keys.hex(code)})`,
    onclick: () => assign(code) }, capContent(label));
  el.style.setProperty("--fs", geo && geo[3] >= 2 ? 1 : Math.max(capScale(label), geo && geo[3] >= 1.5 ? 0.86 : 0));
  if (geo) { const [, x, y, w, hh] = geo; el.style.setProperty("--x", x); el.style.setProperty("--y", y); el.style.setProperty("--w", w); el.style.setProperty("--h", hh); }
  return el;
}
const layerKeys = () => Keys.layerKeys(S.mode * S.st.layers_per_mode, S.st.layers_per_mode);

function renderPal() {
  const box = $("#pal");
  if (!box) return;
  box.className = "pal" + (S.busy ? " busy" : "");
  const kids = [];
  const flow = (codes, cols) => h("div", { class: cols ? "pk-grid" : "pk-flow", style: cols ? { "--cols": cols } : null }, codes.map(c => pk(c)));
  if (S.query) {
    const q = S.query.toLowerCase();
    const all = [...new Set([...Keys.FULL.map(f => f[0]), ...Keys.COMMON, ...layerKeys(), ...Keys.MEDIA, ...Keys.LIGHT, ...Keys.SPECIAL])];
    const hits = all.filter(c => textOf(c).toLowerCase().includes(q) || Keys.hex(c).includes(q));
    kids.push(h("div", { class: "pal-title" }, t("search.results")));
    kids.push(hits.length ? flow(hits) : h("p", { class: "empty" }, t("search.none")));
  } else if (S.cat === "basic") {
    const plate = h("div", { class: "plate", style: { width: "calc(20.25 * var(--pu))", height: "calc(6 * var(--pu))" } },
      Keys.FULL.map(f => pk(f[0], f)));
    kids.push(h("div", { class: "pal-body", id: "pal-body" },
      h("div", { class: "pal-board", id: "pal-board" }, h("div", { class: "pal-title" }, t("cat.basic")), plate),
      h("div", { class: "pal-common" }, h("div", { class: "pal-title" }, t("cat.common")), flow([...Keys.COMMON, ...layerKeys()], 6))));
  } else if (S.cat === "custom") {
    kids.push(h("div", { class: "pal-title" }, t("cat.custom")), renderCustom());
  } else {
    const codes = { media: Keys.MEDIA, light: Keys.LIGHT, special: [...Keys.SPECIAL, ...layerKeys()] }[S.cat];
    kids.push(h("div", { class: "pal-title" }, t("cat." + S.cat)));
    if (S.cat === "light") kids.push(h("p", { class: "pal-hint" }, t("cat.lightHint")));
    kids.push(flow(codes));
  }
  box.replaceChildren(...kids);
}

function renderCustom() {
  const input = h("input", { type: "text", spellcheck: "false", autocomplete: "off", placeholder: t("key.placeholder"), value: S.draft, "aria-label": t("cat.custom") });
  const preview = h("div", { class: "preview muted" });
  const apply = h("button", { class: "btn dark", disabled: true }, t("common.apply"));
  input.addEventListener("input", () => { S.draft = input.value; updatePreview(preview, apply); });
  input.addEventListener("keydown", ev => { if (ev.key === "Enter" && !apply.disabled) apply.click(); });
  apply.addEventListener("click", () => write(S.draft.trim()));
  if (S.draft) updatePreview(preview, apply);
  return h("div", { class: "custom" }, h("div", { class: "row" }, input, apply), preview, h("p", { class: "muted small" }, t("key.help")));
}

async function updatePreview(preview, apply) {
  const text = S.draft.trim();
  const seq = ++S.previewSeq;
  if (!text) { preview.className = "preview muted"; preview.textContent = ""; apply.disabled = true; return; }
  try {
    const k = await api.call("parse_key", { text });
    if (seq !== S.previewSeq) return;
    const same = S.sel != null && k.code === keyAt(S.layer, S.sel).code;
    preview.className = "preview ok";
    preview.textContent = `→ ${textOf(k.code, k.label)}  (${Keys.hex(k.code)})` + (same ? " · " + t("key.same") : "");
    apply.disabled = same || S.sel == null;
  } catch (e) {
    if (seq !== S.previewSeq) return;
    preview.className = "preview bad";
    preview.textContent = errText(e);
    apply.disabled = true;
  }
}

/** Re-render the parts that depend on the selected key / layer, keeping the search box. */
function refreshViews() { renderStage(); renderPal(); fit(); }

function select(slot) {
  S.sel = S.sel === slot ? null : slot;
  refreshViews();
  const el = document.querySelector(`.cap[data-slot="${S.sel}"]`);
  if (el) el.focus({ preventScroll: true });
}

// ---------- writing ----------
function assign(code) {
  if (S.sel == null) { toast(t("key.pickFirst"), true); return; }
  if (keyAt(S.layer, S.sel).code === code) { toast(t("key.same")); return; }
  write("0x" + code.toString(16));
}
async function write(text) {
  if (S.sel == null || S.busy) return;
  const layer = S.layer, slot = S.sel, before = keyAt(layer, slot);
  S.busy = true; renderPal();
  try {
    const k = await api.call("set_key", { layer, slot, text });
    S.st.keys[layer * per() + slot] = k;
    S.draft = "";
    toast(t("toast.saved", { from: textOf(before.code, before.label), to: textOf(k.code, k.label) }));
  } catch (e) { toast(errText(e), true); }
  S.busy = false; refreshViews();
}
async function doReset() {
  if (S.sel == null || S.busy) return;
  const layer = S.layer, slot = S.sel;
  S.busy = true;
  try {
    const k = await api.call("reset_key", { layer, slot });
    S.st.keys[layer * per() + slot] = k;
    toast(t("toast.reset", { to: textOf(k.code, k.label) }));
  } catch (e) { toast(errText(e), true); }
  S.busy = false; refreshViews();
}

// ---------- sizing ----------
const clamp = (v, lo, hi) => Math.max(lo, Math.min(hi, v));
function fit() {
  if (!S.st) return;
  const lay = Layout.forState(S.st);
  const wrap = $("#board-wrap"), kb = $("#kb-case");
  if (wrap && kb) {
    const u = clamp((wrap.clientWidth - 8) / (lay.w + 0.7), 16, 46);
    kb.style.setProperty("--u", u + "px");
    const ex = $("#extra-caps");
    if (ex) ex.parentElement.style.setProperty("--u", clamp(u, 26, 40) + "px");
  }
  const pal = $("#pal");
  if (pal) {
    const body = $("#pal-body");
    let pu = 40;
    if (body) {
      const wide = getComputedStyle(body).flexDirection === "row";
      const avail = body.clientWidth;
      pu = wide ? clamp((avail - 14 - 20 - 8) / (20.25 + 6 * 1.12), 26, 44) : clamp((avail - 8) / 20.25, 26, 44);
    }
    pal.style.setProperty("--pu", pu + "px");
  }
}
new ResizeObserver(() => fit()).observe(document.body);

// ---------- backups drawer ----------
async function renderBackups() {
  const ul = $("#backup-list");
  ul.replaceChildren();
  let list = [];
  try { list = await api.call("list_backups"); } catch (e) { toast(errText(e), true); }
  if (!list.length) { ul.append(h("li", { class: "muted" }, t("backups.empty"))); return; }
  for (const b of list.slice(0, 50)) {
    ul.append(h("li", null,
      h("span", null, new Date(b.time_ms).toLocaleString(i18n.lang)),
      h("button", { class: "btn", disabled: !S.st, onclick: () => restore(b) }, t("backups.restore"))));
  }
}
async function restore(b) {
  if (!confirm(t("backups.confirm"))) return;
  try {
    adopt(await api.call("restore_backup", { path: b.path }));
    toast(t("toast.restored"));
    render(); renderBackups();
  } catch (e) { toast(errText(e), true); }
}

// ---------- wiring ----------
$("#btn-refresh").addEventListener("click", reload);
$("#btn-backups").addEventListener("click", () => { $("#backups").hidden = false; renderBackups(); });
$("#btn-backups-close").addEventListener("click", () => { $("#backups").hidden = true; });
$("#btn-make-backup").addEventListener("click", async () => {
  try { await api.call("make_backup"); toast(t("toast.backup")); renderBackups(); } catch (e) { toast(errText(e), true); }
});
$("#mode-select").addEventListener("change", ev => {
  S.mode = Number(ev.target.value);
  S.layer = S.mode * S.st.layers_per_mode;
  S.sel = null;
  renderHeader(); refreshViews();
});
// Frameless window: the top bar drags the window, double-click maximizes.
(() => {
  const win = window.__TAURI__ && window.__TAURI__.window && window.__TAURI__.window.getCurrentWindow();
  if (!win) return;
  $("#win-ctl").hidden = false;
  $("#win-min").addEventListener("click", () => win.minimize());
  $("#win-max").addEventListener("click", () => win.toggleMaximize());
  $("#win-close").addEventListener("click", () => win.close());
  const interactive = el => el.closest("button, select, input, label, a");
  const bar = $("#titlebar");
  bar.addEventListener("mousedown", ev => {
    if (ev.button !== 0 || interactive(ev.target)) return;
    if (ev.detail === 2) win.toggleMaximize(); else win.startDragging();
  });
})();
$("#lang-select").addEventListener("change", ev => i18n.use(ev.target.value));
document.addEventListener("keydown", ev => {
  if (ev.key === "Escape") {
    if (!$("#backups").hidden) $("#backups").hidden = true;
    else if (S.sel != null) { S.sel = null; refreshViews(); }
  }
});
window.addEventListener("langchange", () => {
  i18n.fillSelect($("#lang-select"));
  document.title = t("app.title");
  render();
  if (!$("#backups").hidden) renderBackups();
});

(async () => {
  await i18n.init();
  await connect();
})();
