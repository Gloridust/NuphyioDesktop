// Tiny i18n: dictionary + data-i18n attributes. Language = saved choice, else browser language.
const I18N = {
  en: {
    "app.title": "NuPhy Desktop",
    "common.close": "Close",
    "common.apply": "Apply",
    "conn.refresh": "Reload",
    "conn.connect": "Connect",
    "conn.connecting": "Connecting…",
    "conn.title": "No keyboard connected",
    "conn.hint": "Plug in a NuPhy keyboard with a USB cable (or its 2.4G dongle), then connect. Everything works offline.",
    "conn.connected": "Connected",
    "err.no_device": "No NuPhy configuration interface found. Use a USB cable or the 2.4G dongle – Bluetooth cannot be configured.",
    "err.permission": "No permission to open the keyboard. Install the udev rule once, then replug the keyboard:",
    "err.not_nuphy": "That device is not a NuPhy keyboard.",
    "err.unsupported_family": "This keyboard family is not supported yet.",
    "err.unknown_model": "Unknown keyboard model.",
    "err.empty_matrix": "The keyboard reported an empty key matrix.",
    "err.not_connected": "Not connected.",
    "err.out_of_range": "That key position does not exist.",
    "err.bad_key": "Cannot understand this key.",
    "err.verify_failed": "The keyboard did not keep the new value.",
    "err.bad_backup": "That backup file is not allowed.",
    "err.restore_failed": "Restore failed.",
    "err.other": "Something went wrong.",
    "dev.fw": "Firmware",
    "dev.mode": "Mode",
    "layers.title": "Layers",
    "layers.mode": "Mode {n}",
    "layers.active": "in use",
    "grid.extra": "Extra keys / knob",
    "grid.legend": "Orange dot = changed from factory. Dim = no key at this position. ▽ = falls through to the layer below.",
    "key.title": "Key",
    "key.none": "Select a key in the grid to edit it.",
    "key.pos": "Layer {layer} · row {row}, col {col}",
    "key.posExtra": "Layer {layer} · extra key {n}",
    "key.current": "Current",
    "key.default": "Factory",
    "key.new": "New function",
    "key.placeholder": "e.g. esc, lctl(c), mo(1), 0x7e0b",
    "key.help": "QMK names or hex codes. Letters, F-keys, media keys, modifiers, layer keys (mo/tg/to…) work; Ctrl+C is lctl(c).",
    "key.quick": "Quick pick",
    "key.reset": "Restore factory",
    "key.same": "Already set to this.",
    "toast.saved": "Saved: {from} → {to}",
    "toast.reset": "Restored factory value: {to}",
    "toast.backup": "Backup created",
    "toast.restored": "Backup restored",
    "backups.open": "Backups",
    "backups.title": "Backups",
    "backups.hint": "The whole key table is saved automatically before every change. Restoring replaces all keys, after saving the current table.",
    "backups.make": "Back up now",
    "backups.restore": "Restore",
    "backups.empty": "No backups yet.",
    "backups.confirm": "Restore this backup? All keys will be replaced (the current table is saved first).",
    "preset.esc": "Esc", "preset.caps": "CapsLock", "preset.pscr": "PrtSc", "preset.del": "Delete",
    "preset.ins": "Insert", "preset.home": "Home", "preset.end": "End", "preset.pgup": "PgUp",
    "preset.pgdn": "PgDn", "preset.lgui": "Win / Super", "preset.mute": "Mute", "preset.vold": "Vol −",
    "preset.volu": "Vol +", "preset.mply": "Play/Pause", "preset.trns": "Transparent", "preset.no": "None",
    "demo.banner": "Demo mode – not connected to a keyboard (shown because this page runs outside the app)."
  },
  zh: {
    "app.title": "NuPhy 桌面版",
    "common.close": "关闭",
    "common.apply": "应用",
    "conn.refresh": "重新读取",
    "conn.connect": "连接",
    "conn.connecting": "正在连接…",
    "conn.title": "未连接键盘",
    "conn.hint": "用 USB 线（或 2.4G 接收器）连接 NuPhy 键盘后点击连接。全程离线可用。",
    "conn.connected": "已连接",
    "err.no_device": "没有找到 NuPhy 的配置接口。请用 USB 线或 2.4G 接收器连接，蓝牙模式无法配置。",
    "err.permission": "没有权限打开键盘。请先安装一次 udev 规则，然后重新插拔键盘：",
    "err.not_nuphy": "该设备不是 NuPhy 键盘。",
    "err.unsupported_family": "暂不支持这个系列的键盘。",
    "err.unknown_model": "未知的键盘型号。",
    "err.empty_matrix": "键盘返回了空的键位表。",
    "err.not_connected": "尚未连接。",
    "err.out_of_range": "不存在这个键位。",
    "err.bad_key": "无法识别这个按键。",
    "err.verify_failed": "键盘没有保存新的设置。",
    "err.bad_backup": "不允许使用这个备份文件。",
    "err.restore_failed": "还原失败。",
    "err.other": "出错了。",
    "dev.fw": "固件",
    "dev.mode": "模式",
    "layers.title": "层",
    "layers.mode": "模式 {n}",
    "layers.active": "使用中",
    "grid.extra": "额外按键 / 旋钮",
    "grid.legend": "橙色圆点 = 与出厂设置不同；灰暗 = 此处没有按键；▽ = 透传到下一层。",
    "key.title": "按键",
    "key.none": "在键盘图中点选一个键来编辑。",
    "key.pos": "第 {layer} 层 · 第 {row} 行 第 {col} 列",
    "key.posExtra": "第 {layer} 层 · 额外按键 {n}",
    "key.current": "当前",
    "key.default": "出厂",
    "key.new": "新的功能",
    "key.placeholder": "例如 esc、lctl(c)、mo(1)、0x7e0b",
    "key.help": "可输入 QMK 名称或十六进制码：字母、F 键、多媒体键、修饰键、层切换（mo/tg/to…）都可以；Ctrl+C 写作 lctl(c)。",
    "key.quick": "快捷选择",
    "key.reset": "恢复出厂",
    "key.same": "已经是这个功能了。",
    "toast.saved": "已保存：{from} → {to}",
    "toast.reset": "已恢复出厂值：{to}",
    "toast.backup": "已创建备份",
    "toast.restored": "已还原备份",
    "backups.open": "备份",
    "backups.title": "备份",
    "backups.hint": "每次改键前都会自动备份整张键位表。还原会替换所有按键，并先保存当前的键位表。",
    "backups.make": "立即备份",
    "backups.restore": "还原",
    "backups.empty": "还没有备份。",
    "backups.confirm": "还原这份备份吗？所有按键都会被替换（会先保存当前键位表）。",
    "preset.esc": "Esc", "preset.caps": "大写锁定", "preset.pscr": "截屏键", "preset.del": "删除",
    "preset.ins": "插入", "preset.home": "Home", "preset.end": "End", "preset.pgup": "上翻页",
    "preset.pgdn": "下翻页", "preset.lgui": "Win / Super", "preset.mute": "静音", "preset.vold": "音量 −",
    "preset.volu": "音量 +", "preset.mply": "播放/暂停", "preset.trns": "透传", "preset.no": "无功能",
    "demo.banner": "演示模式：未连接键盘（因为此页面在应用之外打开）。"
  }
};

const i18n = {
  lang: "en",
  init() {
    let saved = null;
    try { saved = localStorage.getItem("lang"); } catch (_) {}
    this.lang = saved && I18N[saved] ? saved : (navigator.language || "en").toLowerCase().startsWith("zh") ? "zh" : "en";
    this.apply();
  },
  set(lang) {
    this.lang = lang;
    try { localStorage.setItem("lang", lang); } catch (_) {}
    this.apply();
  },
  toggle() { this.set(this.lang === "zh" ? "en" : "zh"); },
  t(key, vars) {
    let s = (I18N[this.lang] && I18N[this.lang][key]) || I18N.en[key] || key;
    if (vars) for (const k in vars) s = s.split("{" + k + "}").join(vars[k]);
    return s;
  },
  apply() {
    document.documentElement.lang = this.lang === "zh" ? "zh-CN" : "en";
    document.querySelectorAll("[data-i18n]").forEach(el => { el.textContent = this.t(el.dataset.i18n); });
    window.dispatchEvent(new Event("langchange"));
  }
};
