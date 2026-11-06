// Backend bridge. Inside the Tauri app it calls the Rust commands; in a plain
// browser it falls back to a small in-memory demo keyboard so the UI can be previewed.
const api = (() => {
  const tauri = window.__TAURI__ && window.__TAURI__.core;
  if (tauri) {
    return { demo: false, call: (cmd, args) => tauri.invoke(cmd, args || {}) };
  }

  // ---- demo keyboard (Node 75 LP-like: 8 layers, 6x16 + 5 extra) ----
  const ROWS = 6, COLS = 16, EXTRA = 5, LAYERS = 8, PER = 4, SLOTS = ROWS * COLS + EXTRA;
  const NAMES = { no: 0, trns: 1, esc: 0x29, caps: 0x39, pscr: 0x46, ins: 0x49, del: 0x4c, home: 0x4a, end: 0x4d,
    pgup: 0x4b, pgdn: 0x4e, lgui: 0xe3, mute: 0xa8, vold: 0xaa, volu: 0xa9, mply: 0xae, mnxt: 0xab };
  const REG = {};
  const label = c => { for (const k in NAMES) if (NAMES[k] === c) return k.toUpperCase(); return REG[c] || (c ? "0x" + c.toString(16).padStart(4, "0") : "NO"); };
  const rows = [
    "ESC F1 F2 F3 F4 F5 F6 F7 F8 F9 F10 F11 F12 PSCR INS DEL",
    "GRV 1 2 3 4 5 6 7 8 9 0 MINS EQL BSPC . HOME",
    "TAB Q W E R T Y U I O P LBRC RBRC BSLS . END",
    "CAPS A S D F G H J K L SCLN QUOT . ENT . PGUP",
    "LSFT Z X C V B N M COMM DOT SLSH . RSFT UP . PGDN",
    "LCTL LGUI LALT . . SPC . . . RALT MO(5) RCTL LEFT DOWN . RGHT"
  ].map(r => r.split(" "));
  const hash = s => [...s].reduce((a, ch) => (a * 31 + ch.charCodeAt(0)) & 0x7fff, 7) + 0x100;
  const codeOfName = n => { const k = n.toLowerCase(); if (k in NAMES) return NAMES[k]; if (n === ".") return 0; const c = hash(n); REG[c] = n.toUpperCase(); return c; };
  const defaults = [];
  for (let l = 0; l < LAYERS; l++) {
    for (let r = 0; r < ROWS; r++) for (let c = 0; c < COLS; c++)
      defaults.push(l === 4 ? codeOfName(rows[r][c]) : (l === 5 || l === 0 || l === 1 || l === 2 || l === 3 || l > 5) && l !== 4 ? (rows[r][c] === "." ? 0 : 1) : 0);
    [0x7e1b, 0xa8, 0xab, 0xaa, 0xa9].forEach(x => defaults.push(l === 4 ? x : 1));
  }
  let keys = defaults.slice();
  keys[4 * SLOTS + 13] = NAMES.lgui; // like the real keyboard: PrtSc position set to Win
  let backups = [];
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const dto = () => ({
    path: "/dev/hidraw6 (demo)", name: "NuPhy Node 75 LP (demo)", model: "NuPhy Node 75 LP", vid: 0x19f5, pid: 0x1030,
    firmware: "1.0.10.6", current_mode: 1, layer_count: LAYERS, layers_per_mode: PER, rows: ROWS, cols: COLS, extra: EXTRA,
    active_layer: 4, slots_per_layer: SLOTS, keys: keys.map(c => ({ code: c, label: label(c) })), defaults: defaults.slice()
  });
  const parse = text => {
    const t = text.trim();
    if (!t) throw { code: "bad_key", detail: "empty" };
    if (/^0x[0-9a-f]{1,4}$/i.test(t)) { const c = parseInt(t, 16); return { code: c, label: label(c) }; }
    if (/^[a-z0-9_()]+$/i.test(t) && !/^bad/i.test(t)) { const c = codeOfName(t); return { code: c, label: label(c) }; }
    throw { code: "bad_key", detail: "unknown key '" + t + "'" };
  };
  const handlers = {
    async connect() { await sleep(250); return dto(); },
    async disconnect() {}, async refresh() { return dto(); },
    async parse_key({ text }) { return parse(text); },
    async set_key({ layer, slot, text }) { const k = parse(text); backups.unshift(Date.now()); keys[layer * SLOTS + slot] = k.code; return k; },
    async reset_key({ layer, slot }) { const c = defaults[layer * SLOTS + slot]; backups.unshift(Date.now()); keys[layer * SLOTS + slot] = c; return { code: c, label: label(c) }; },
    async list_backups() { return backups.map(t => ({ path: "demo-" + t, name: "1030-" + t + ".bin", time_ms: t })); },
    async make_backup() { backups.unshift(Date.now()); return "demo"; },
    async restore_backup() { return dto(); }
  };
  return { demo: true, call: (cmd, args) => handlers[cmd](args || {}) };
})();
