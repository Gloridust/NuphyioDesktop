// i18n: languages are listed in locales/index.js, strings live in locales/<code>.js.
// Files are plain <script>s (not fetch) so the page also works when opened from disk.
// Missing strings fall back to English.
"use strict";

const I18N_DICTS = {};
let I18N_LIST = [];
function I18N_REGISTER(code, dict) { I18N_DICTS[code] = dict; }
function I18N_LANGUAGES(list) { I18N_LIST = list; }

const i18n = {
  pref: "auto",   // "auto" or a language code
  lang: "en",     // resolved language code

  loadScript(src) {
    return new Promise((resolve, reject) => {
      const s = document.createElement("script");
      s.src = src;
      s.onload = resolve;
      s.onerror = () => reject(new Error("cannot load " + src));
      document.head.append(s);
    });
  },

  async ensure(code) {
    if (!I18N_DICTS[code]) await this.loadScript("locales/" + code + ".js");
  },

  /** Best language for the system: exact tag, then same base language, else English. */
  systemLang() {
    const codes = I18N_LIST.map(l => l.code);
    const wanted = (navigator.languages && navigator.languages.length ? navigator.languages : [navigator.language || "en"]);
    for (const w of wanted) {
      const exact = codes.find(c => c.toLowerCase() === w.toLowerCase());
      if (exact) return exact;
    }
    for (const w of wanted) {
      const base = w.toLowerCase().split("-")[0];
      const same = codes.find(c => c.toLowerCase().split("-")[0] === base);
      if (same) return same;
    }
    return "en";
  },

  async init() {
    await this.loadScript("locales/index.js");
    let saved = null;
    try { saved = localStorage.getItem("lang"); } catch (_) {}
    const known = saved === "auto" || I18N_LIST.some(l => l.code === saved);
    await this.use(known ? saved : "auto", false);
  },

  async use(pref, save = true) {
    const code = pref === "auto" ? this.systemLang() : pref;
    await this.ensure("en");
    try { await this.ensure(code); } catch (_) {}
    this.pref = pref;
    this.lang = I18N_DICTS[code] ? code : "en";
    if (save) { try { localStorage.setItem("lang", pref); } catch (_) {} }
    this.apply();
  },

  name(code) {
    const l = I18N_LIST.find(x => x.code === code);
    return l ? l.name : code;
  },

  t(key, vars) {
    let s = (I18N_DICTS[this.lang] || {})[key];
    if (s == null) s = (I18N_DICTS.en || {})[key];
    if (s == null) s = key;
    if (vars) for (const k in vars) s = s.split("{" + k + "}").join(vars[k]);
    return s;
  },

  /** Fill a <select> with "follow system" + every listed language. */
  fillSelect(sel) {
    sel.replaceChildren();
    const auto = document.createElement("option");
    auto.value = "auto";
    auto.textContent = this.t("lang.auto", { name: this.name(this.systemLang()) });
    sel.append(auto);
    for (const l of I18N_LIST) {
      const o = document.createElement("option");
      o.value = l.code;
      o.textContent = l.name;
      o.lang = l.code;
      sel.append(o);
    }
    sel.value = this.pref;
  },

  apply() {
    document.documentElement.lang = this.lang;
    document.querySelectorAll("[data-i18n]").forEach(el => { el.textContent = this.t(el.dataset.i18n); });
    document.querySelectorAll("[data-i18n-title]").forEach(el => { el.title = this.t(el.dataset.i18nTitle); el.setAttribute("aria-label", el.title); });
    window.dispatchEvent(new Event("langchange"));
  }
};
