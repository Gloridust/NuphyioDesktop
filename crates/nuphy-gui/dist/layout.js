// Keyboard drawing geometry. Uses the model's real key positions (layouts.js) when they
// can be matched to the keyboard's matrix, otherwise a plain grid of the matrix.
"use strict";

const Layout = (() => {
  const GAP = 0.04; // the web app adds 0.04 key widths between neighbouring keys

  /** Positions with the inter-key gap applied (same rule as the web app). */
  function geometry(entries) {
    let a = 0;
    return entries.map(e => {
      if (e.x === 0) a = 0;
      const g = { x: (e.x || 0) + GAP * a, y: e.y || 0, w: e.w || 1, h: e.h || 1 };
      a++;
      return g;
    });
  }

  /** Slots (per row) that hold a key in at least one layer of the factory map. */
  function usedSlots(st) {
    const per = st.slots_per_layer, used = new Set();
    for (let l = 0; l < st.layer_count; l++)
      for (let s = 0; s < per; s++) if (st.defaults[l * per + s] !== 0) used.add(s);
    return used;
  }

  /**
   * Match the layout to the matrix: row by row, the drawn keys take the row's used
   * matrix slots in order; keys off the board (knob, hidden extras) take the extra slots.
   * Returns null when the counts disagree or the plain keys don't match the factory map.
   */
  function fromModel(st) {
    const L = (typeof LAYOUTS !== "undefined") && LAYOUTS[st.model];
    if (!L) return null;
    const rows = [];
    L.forEach((e, i) => { if (i === 0 || e.nl) rows.push([]); rows[rows.length - 1].push(i); });
    if (rows.length !== st.rows) return null;

    const used = usedSlots(st), slotOf = new Map(), extras = [];
    for (let r = 0; r < st.rows; r++) {
      const want = [];
      for (let c = 0; c < st.cols; c++) if (used.has(r * st.cols + c)) want.push(r * st.cols + c);
      const onBoard = i => (L[i].y || 0) >= 0 && L[i].x != null;
      const variants = [rows[r].filter(i => onBoard(i) && !L[i].hid), rows[r].filter(onBoard)];
      const pick = variants.find(v => v.length === want.length);
      if (!pick) return null;
      pick.forEach((i, k) => slotOf.set(i, want[k]));
      rows[r].forEach(i => { if (!slotOf.has(i)) extras.push(i); });
    }
    extras.forEach((i, k) => { if (k < st.extra) slotOf.set(i, st.rows * st.cols + k); });

    // Check the plain HID keys against each mode's base layer.
    const per = st.slots_per_layer;
    let best = 0, compared = 0;
    for (let m = 0; m * st.layers_per_mode < st.layer_count; m++) {
      const base = m * st.layers_per_mode * per;
      let ok = 0, n = 0;
      for (const [i, slot] of slotOf) {
        const c = L[i].c, d = st.defaults[base + slot];
        if (c == null || d > 0xff) continue;
        n++;
        if (c === d) ok++;
      }
      if (n && ok / n > best) { best = ok / n; compared = n; }
    }
    if (compared < 8 || best < 0.75) return null;

    // Extra slots (knob, side keys) are listed separately rather than drawn on the board.
    const geo = geometry(L), keys = [], matrix = st.rows * st.cols;
    for (const [i, slot] of slotOf) if (!L[i].hid && slot < matrix) keys.push(Object.assign({ slot }, geo[i]));
    return finish(keys, true, st);
  }

  /** Fallback: the raw matrix, one cell per used slot, extras on their own row. */
  function grid(st) {
    const used = usedSlots(st), keys = [], step = 1 + GAP;
    for (let r = 0; r < st.rows; r++)
      for (let c = 0; c < st.cols; c++) {
        const slot = r * st.cols + c;
        if (used.has(slot)) keys.push({ slot, x: c * step, y: r * step, w: 1, h: 1 });
      }
    return finish(keys, false, st);
  }

  function finish(keys, real, st) {
    const minX = Math.min(...keys.map(k => k.x)), minY = Math.min(...keys.map(k => k.y));
    keys.forEach(k => { k.x -= minX; k.y -= minY; });
    return {
      keys, real,
      extras: Array.from({ length: st.extra }, (_, k) => st.rows * st.cols + k),
      w: Math.max(...keys.map(k => k.x + k.w)),
      h: Math.max(...keys.map(k => k.y + k.h))
    };
  }

  const cache = new Map();
  function forState(st) {
    const key = st.model + "/" + st.pid + "/" + st.defaults.length;
    if (!cache.has(key)) cache.set(key, fromModel(st) || grid(st));
    return cache.get(key);
  }

  return { forState, fromModel, grid };
})();
