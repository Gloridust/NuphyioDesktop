#!/usr/bin/env python3
"""Generate crates/nuphy-gui/dist/layouts.js from the NuPhyIO web bundle.

The web app ships one key-position table per keyboard model (webpack module 97075):
entries like {keyCode: KC_X, x, y, w, h, hidden, beginNewLine} in key units. This
script parses them statically (nothing from the bundle is executed) and writes only
the geometry plus the plain HID code of each key, which the GUI uses to check that a
layout really matches the keyboard's matrix before drawing it.

Usage:
    curl -sO https://drive.nuphyio.com/static/js/main.<hash>.js
    python3 tools/gen_layouts.py main.<hash>.js > crates/nuphy-gui/dist/layouts.js
"""
import json
import re
import sys

LAYOUT_MODULE = 97075

# Web keycode names that are plain HID usages (identical in the web app and in firmware).
BASIC = {}
for i, ch in enumerate("ABCDEFGHIJKLMNOPQRSTUVWXYZ"):
    BASIC[ch] = 0x04 + i
for i, ch in enumerate("1234567890"):
    BASIC[ch] = 0x1E + i
for i in range(12):
    BASIC[f"F{i + 1}"] = 0x3A + i
for i in range(12):
    BASIC[f"F{i + 13}"] = 0x68 + i
for i, ch in enumerate("1234567890"):
    BASIC[f"KP_{ch}"] = 0x59 + i
for names, code in [
    ("ENTER ENT", 0x28), ("ESCAPE ESC", 0x29), ("BACKSPACE BSPC", 0x2A), ("TAB", 0x2B),
    ("SPACE SPC", 0x2C), ("MINUS MINS", 0x2D), ("EQUAL EQL", 0x2E), ("LEFT_BRACKET LBRC", 0x2F),
    ("RIGHT_BRACKET RBRC", 0x30), ("BACKSLASH BSLS", 0x31), ("NONUS_HASH NUHS", 0x32),
    ("SEMICOLON SCLN", 0x33), ("QUOTE QUOT", 0x34), ("GRAVE GRV", 0x35), ("COMMA COMM", 0x36),
    ("DOT", 0x37), ("SLASH SLSH", 0x38), ("CAPS_LOCK CAPS", 0x39), ("PRINT_SCREEN PSCR", 0x46),
    ("SCROLL_LOCK SCRL", 0x47), ("PAUSE", 0x48), ("INSERT INS", 0x49), ("HOME", 0x4A),
    ("PAGE_UP PGUP", 0x4B), ("DELETE DEL", 0x4C), ("END", 0x4D), ("PAGE_DOWN PGDN", 0x4E),
    ("RIGHT RGHT", 0x4F), ("LEFT", 0x50), ("DOWN", 0x51), ("UP", 0x52), ("NUM_LOCK NUM", 0x53),
    ("KP_SLASH PSLS", 0x54), ("KP_ASTERISK PAST", 0x55), ("KP_MINUS PMNS", 0x56),
    ("KP_PLUS PPLS", 0x57), ("KP_ENTER PENT", 0x58), ("KP_DOT PDOT", 0x63),
    ("NONUS_BACKSLASH NUBS", 0x64), ("APPLICATION APP", 0x65), ("RO INT1", 0x87),
    ("KANA INT2", 0x88), ("JYEN INT3", 0x89), ("HENK INT4", 0x8A), ("MHEN INT5", 0x8B),
    ("LEFT_CTRL LCTL", 0xE0), ("LEFT_SHIFT LSFT", 0xE1), ("LEFT_ALT LALT", 0xE2),
    ("LEFT_GUI LGUI", 0xE3), ("RIGHT_CTRL RCTL", 0xE4), ("RIGHT_SHIFT RSFT", 0xE5),
    ("RIGHT_ALT RALT", 0xE6), ("RIGHT_GUI RGUI", 0xE7),
]:
    for n in names.split():
        BASIC[n] = code


def module(src, mid):
    i = src.find(f"{mid}:function(e,t,n)")
    if i < 0:
        sys.exit(f"module {mid} not found")
    return src[i:match_bracket(src, src.find("{", i)) + 1]


def match_bracket(s, i):
    close = {"[": "]", "{": "}"}
    stack, in_str, k = [], None, i
    while k < len(s):
        c = s[k]
        if in_str:
            if c == "\\":
                k += 1
            elif c == in_str:
                in_str = None
        elif c in "\"'`":
            in_str = c
        elif c in "[{":
            stack.append(close[c])
        elif c in "]}":
            stack.pop()
            if not stack:
                return k
        k += 1
    raise ValueError("unbalanced")


def value(v):
    v = v.strip()
    if v in ("!0", "true"):
        return True
    if v in ("!1", "false"):
        return False
    try:
        return float(v)
    except ValueError:
        return None


def parse_entry(body):
    e = {}
    for part in re.split(r",(?![^\[{]*[\]}])", body):
        if ":" not in part:
            continue
        k, v = part.split(":", 1)
        k = k.strip()
        if k == "keyCode":
            m = re.search(r"KC_(\w+)", v)
            e["kc"] = m.group(1) if m else None
        elif k in ("x", "y", "w", "h", "hidden", "beginNewLine"):
            e[k] = value(v)
    return e


def arrays(mod):
    out = {}
    for m in re.finditer(r"([\w$]+)=\[\{keyCode:", mod):
        start = m.end() - len("[{keyCode:")
        body = mod[start + 1:match_bracket(mod, start)]
        entries, k = [], 0
        while k < len(body):
            if body[k] == "{":
                end = match_bracket(body, k)
                entries.append(parse_entry(body[k + 1:end]))
                k = end + 1
            else:
                k += 1
        out[m.group(1)] = entries
    return out


def model_map(src):
    """Model name (as in the device table) -> exported layout name."""
    res = {}
    for m in re.finditer(r"keys:([\w$]+)\.([\w$]+)", src):
        alias, exp = m.group(1), m.group(2)
        head = src.rfind(":function(e,t,n){", 0, m.start())
        if not re.search(r"[,\s]" + re.escape(alias) + rf"=n\({LAYOUT_MODULE}\)", src[head:m.start()]):
            continue
        prev = list(re.finditer(r"ui\.(\w+)", src[max(head, m.start() - 3000):m.start()]))
        if prev:
            res.setdefault(prev[-1].group(1), exp)
    return res


def compact(e):
    out = {}
    for k in ("x", "y", "w", "h"):
        v = e.get(k)
        if v is not None and not (k in ("w", "h") and v == 1):
            out[k] = round(v, 3)
    if e.get("hidden"):
        out["hid"] = 1
    if e.get("beginNewLine"):
        out["nl"] = 1
    code = BASIC.get(e.get("kc") or "")
    if code is not None:
        out["c"] = code
    return out


def main():
    src = open(sys.argv[1], encoding="utf-8", errors="replace").read()
    mod = module(src, LAYOUT_MODULE)
    arrs = arrays(mod)
    exports = dict(re.findall(r"([\w$]+):function\(\)\{return ([\w$]+)\}", mod))
    layouts = {}
    for model, exp in sorted(model_map(src).items()):
        var = exports.get(exp)
        if var in arrs:
            layouts[model] = [compact(e) for e in arrs[var]]
    print("// Generated by tools/gen_layouts.py from the NuPhyIO web app's key-position tables.")
    print("// Units are key widths; only geometry and plain HID codes are kept.")
    print("const LAYOUTS = {")
    for i, (model, entries) in enumerate(layouts.items()):
        sep = "," if i < len(layouts) - 1 else ""
        print(f'  "{model}": [')
        for j, e in enumerate(entries):
            print("    " + json.dumps(e, separators=(",", ":")) + ("," if j < len(entries) - 1 else ""))
        print(f"  ]{sep}")
    print("};")
    print(f"{len(layouts)} layouts", file=sys.stderr)


if __name__ == "__main__":
    main()
