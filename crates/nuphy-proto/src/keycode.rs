//! QMK-style 16-bit keycodes, as stored in the key matrix (big endian).
//!
//! Verified on a Node 75 LP (fw 1.0.10.6): the matrix holds QMK keycodes
//! (`0x0001` transparent, `0x5221` = MO(1), `0x00A8` = mute …). The web app
//! converts some ranges between its own QMK version and the firmware's; only
//! the ranges named below are known to be identical, so [`KeyCode::parse`]
//! only produces those. Other values can still be written as raw hex.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyCode(pub u16);

pub const KC_NO: KeyCode = KeyCode(0x0000);
pub const KC_TRNS: KeyCode = KeyCode(0x0001);

const QK_MODS: u16 = 0x0100;
const QK_MODS_MAX: u16 = 0x1FFF;
const QK_TO: u16 = 0x5200;
const QK_MOMENTARY: u16 = 0x5220;
const QK_DEF_LAYER: u16 = 0x5240;
const QK_TOGGLE_LAYER: u16 = 0x5260;
const QK_ONE_SHOT_LAYER: u16 = 0x5280;
const QK_LAYER_TAP_TOGGLE: u16 = 0x52C0;
const QK_LIGHTING: u16 = 0x7800;
const QK_KB: u16 = 0x7E00;

/// (name, code) for the basic page. First name is the canonical one.
const BASIC: &[(&str, u8)] = &[
    ("NO", 0x00), ("TRNS", 0x01),
    ("A", 0x04), ("B", 0x05), ("C", 0x06), ("D", 0x07), ("E", 0x08), ("F", 0x09), ("G", 0x0A),
    ("H", 0x0B), ("I", 0x0C), ("J", 0x0D), ("K", 0x0E), ("L", 0x0F), ("M", 0x10), ("N", 0x11),
    ("O", 0x12), ("P", 0x13), ("Q", 0x14), ("R", 0x15), ("S", 0x16), ("T", 0x17), ("U", 0x18),
    ("V", 0x19), ("W", 0x1A), ("X", 0x1B), ("Y", 0x1C), ("Z", 0x1D),
    ("1", 0x1E), ("2", 0x1F), ("3", 0x20), ("4", 0x21), ("5", 0x22), ("6", 0x23), ("7", 0x24),
    ("8", 0x25), ("9", 0x26), ("0", 0x27),
    ("ENT", 0x28), ("ESC", 0x29), ("BSPC", 0x2A), ("TAB", 0x2B), ("SPC", 0x2C), ("MINS", 0x2D),
    ("EQL", 0x2E), ("LBRC", 0x2F), ("RBRC", 0x30), ("BSLS", 0x31), ("NUHS", 0x32), ("SCLN", 0x33),
    ("QUOT", 0x34), ("GRV", 0x35), ("COMM", 0x36), ("DOT", 0x37), ("SLSH", 0x38), ("CAPS", 0x39),
    ("F1", 0x3A), ("F2", 0x3B), ("F3", 0x3C), ("F4", 0x3D), ("F5", 0x3E), ("F6", 0x3F),
    ("F7", 0x40), ("F8", 0x41), ("F9", 0x42), ("F10", 0x43), ("F11", 0x44), ("F12", 0x45),
    ("PSCR", 0x46), ("SCRL", 0x47), ("PAUS", 0x48), ("INS", 0x49), ("HOME", 0x4A), ("PGUP", 0x4B),
    ("DEL", 0x4C), ("END", 0x4D), ("PGDN", 0x4E), ("RGHT", 0x4F), ("LEFT", 0x50), ("DOWN", 0x51),
    ("UP", 0x52), ("NUM", 0x53), ("PSLS", 0x54), ("PAST", 0x55), ("PMNS", 0x56), ("PPLS", 0x57),
    ("PENT", 0x58), ("P1", 0x59), ("P2", 0x5A), ("P3", 0x5B), ("P4", 0x5C), ("P5", 0x5D),
    ("P6", 0x5E), ("P7", 0x5F), ("P8", 0x60), ("P9", 0x61), ("P0", 0x62), ("PDOT", 0x63),
    ("NUBS", 0x64), ("APP", 0x65), ("PWR", 0x66), ("PEQL", 0x67),
    ("F13", 0x68), ("F14", 0x69), ("F15", 0x6A), ("F16", 0x6B), ("F17", 0x6C), ("F18", 0x6D),
    ("F19", 0x6E), ("F20", 0x6F), ("F21", 0x70), ("F22", 0x71), ("F23", 0x72), ("F24", 0x73),
    ("SYSTEM_POWER", 0xA5), ("SYSTEM_SLEEP", 0xA6), ("SYSTEM_WAKE", 0xA7),
    ("MUTE", 0xA8), ("VOLU", 0xA9), ("VOLD", 0xAA), ("MNXT", 0xAB), ("MPRV", 0xAC), ("MSTP", 0xAD),
    ("MPLY", 0xAE), ("MSEL", 0xAF), ("EJCT", 0xB0), ("MAIL", 0xB1), ("CALC", 0xB2), ("MYCM", 0xB3),
    ("WSCH", 0xB4), ("WHOM", 0xB5), ("WBAK", 0xB6), ("WFWD", 0xB7), ("WSTP", 0xB8), ("WREF", 0xB9),
    ("WFAV", 0xBA), ("MFFD", 0xBB), ("MRWD", 0xBC), ("BRIU", 0xBD), ("BRID", 0xBE),
    ("LCTL", 0xE0), ("LSFT", 0xE1), ("LALT", 0xE2), ("LGUI", 0xE3),
    ("RCTL", 0xE4), ("RSFT", 0xE5), ("RALT", 0xE6), ("RGUI", 0xE7),
];

/// Extra spellings accepted by the parser.
const ALIASES: &[(&str, &str)] = &[
    ("ENTER", "ENT"), ("ESCAPE", "ESC"), ("BACKSPACE", "BSPC"), ("SPACE", "SPC"), ("MINUS", "MINS"),
    ("EQUAL", "EQL"), ("CAPSLOCK", "CAPS"), ("DELETE", "DEL"), ("INSERT", "INS"), ("RIGHT", "RGHT"),
    ("PAGEUP", "PGUP"), ("PAGEDOWN", "PGDN"), ("TRANSPARENT", "TRNS"), ("NONE", "NO"),
    ("LCTRL", "LCTL"), ("LSHIFT", "LSFT"), ("LWIN", "LGUI"), ("LCMD", "LGUI"), ("RCTRL", "RCTL"),
    ("RSHIFT", "RSFT"), ("RWIN", "RGUI"), ("RCMD", "RGUI"), ("GRAVE", "GRV"), ("COMMA", "COMM"),
    ("SLASH", "SLSH"), ("QUOTE", "QUOT"), ("SEMICOLON", "SCLN"), ("BACKSLASH", "BSLS"),
    ("PRINTSCREEN", "PSCR"), ("MENU", "APP"),
];

/// Modifier bits in QK_MODS keycodes (bit 4 = right-hand).
const MOD_NAMES: &[(&str, u16)] = &[("C", 0x01), ("S", 0x02), ("A", 0x04), ("G", 0x08)];

fn basic_name(code: u8) -> Option<&'static str> {
    BASIC.iter().find(|(_, c)| *c == code).map(|(n, _)| *n)
}

fn basic_code(name: &str) -> Option<u8> {
    let n = name.strip_prefix("KC_").unwrap_or(name);
    let n = ALIASES.iter().find(|(a, _)| *a == n).map(|(_, t)| *t).unwrap_or(n);
    BASIC.iter().find(|(b, _)| *b == n).map(|(_, c)| *c)
}

impl KeyCode {
    pub fn from_be(b: [u8; 2]) -> Self {
        Self(u16::from_be_bytes(b))
    }

    pub fn to_be(self) -> [u8; 2] {
        self.0.to_be_bytes()
    }

    /// Parse `KC_A`, `a`, `esc`, `MO(1)`, `TG(2)`, `LCTL(C)`, `0x5221`, …
    pub fn parse(s: &str) -> Result<Self, String> {
        let u = s.trim().to_ascii_uppercase();
        if let Some(h) = u.strip_prefix("0X") {
            return u16::from_str_radix(h, 16).map(KeyCode).map_err(|e| format!("bad hex '{s}': {e}"));
        }
        if let Some((f, rest)) = u.split_once('(') {
            let arg = rest.strip_suffix(')').ok_or_else(|| format!("missing ')' in '{s}'"))?;
            let layer = |base: u16| -> Result<KeyCode, String> {
                let n: u16 = arg.parse().map_err(|_| format!("bad layer in '{s}'"))?;
                if n > 0x1F {
                    return Err(format!("layer {n} out of range"));
                }
                Ok(KeyCode(base | n))
            };
            return match f {
                "TO" => layer(QK_TO),
                "MO" => layer(QK_MOMENTARY),
                "DF" => layer(QK_DEF_LAYER),
                "TG" => layer(QK_TOGGLE_LAYER),
                "OSL" => layer(QK_ONE_SHOT_LAYER),
                "TT" => layer(QK_LAYER_TAP_TOGGLE),
                _ => {
                    // Modifier wrappers: LCTL(x), LSFT(x), RALT(x), C_S(x) …
                    let (right, mods) = match f {
                        "LCTL" | "C" => (false, 0x01),
                        "LSFT" | "S" => (false, 0x02),
                        "LALT" | "A" => (false, 0x04),
                        "LGUI" | "G" => (false, 0x08),
                        "RCTL" => (true, 0x01),
                        "RSFT" => (true, 0x02),
                        "RALT" => (true, 0x04),
                        "RGUI" => (true, 0x08),
                        _ => return Err(format!("unknown function '{f}'")),
                    };
                    let inner = KeyCode::parse(arg)?;
                    let (inner_mods, base) = if (QK_MODS..=QK_MODS_MAX).contains(&inner.0) {
                        ((inner.0 >> 8) & 0x1F, inner.0 & 0xFF)
                    } else if inner.0 <= 0xFF {
                        (0, inner.0)
                    } else {
                        return Err(format!("'{arg}' can't take a modifier"));
                    };
                    let m = inner_mods | mods | if right { 0x10 } else { 0 };
                    Ok(KeyCode(m << 8 | base))
                }
            };
        }
        basic_code(&u).map(|c| KeyCode(u16::from(c))).ok_or_else(|| format!("unknown key '{s}'"))
    }

    pub fn is_transparent(self) -> bool {
        self == KC_TRNS
    }
}

impl fmt::Display for KeyCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let c = self.0;
        match c {
            0x0000..=0x00FF => match basic_name(c as u8) {
                Some(n) => write!(f, "{n}"),
                None => write!(f, "{c:#06x}"),
            },
            QK_MODS..=QK_MODS_MAX => {
                let m = (c >> 8) & 0x1F;
                let side = if m & 0x10 != 0 { "R" } else { "L" };
                let names: Vec<&str> =
                    MOD_NAMES.iter().filter(|(_, b)| m & b != 0).map(|(n, _)| *n).collect();
                let inner = KeyCode(c & 0xFF);
                write!(f, "{side}{}({inner})", names.join(""))
            }
            0x5200..=0x521F => write!(f, "TO({})", c & 0x1F),
            0x5220..=0x523F => write!(f, "MO({})", c & 0x1F),
            0x5240..=0x525F => write!(f, "DF({})", c & 0x1F),
            0x5260..=0x527F => write!(f, "TG({})", c & 0x1F),
            0x5280..=0x529F => write!(f, "OSL({})", c & 0x1F),
            0x52C0..=0x52DF => write!(f, "TT({})", c & 0x1F),
            0x7800..=0x78FF => write!(f, "LIGHT({:#04x})", c - QK_LIGHTING),
            0x7E00..=0x7EFF => write!(f, "NUPHY({:#04x})", c - QK_KB),
            _ => write!(f, "{c:#06x}"),
        }
    }
}

/// Split a raw region into keycodes (a trailing odd byte is ignored).
pub fn parse_codes(raw: &[u8]) -> Vec<KeyCode> {
    raw.as_chunks::<2>().0.iter().map(|c| KeyCode::from_be(*c)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_real_node75_values() {
        let raw = [0x00, 0x29, 0x00, 0x69, 0x7E, 0x06, 0x00, 0x01, 0x52, 0x21, 0x78, 0x2A, 0x00, 0xA8];
        let k: Vec<String> = parse_codes(&raw).iter().map(|k| k.to_string()).collect();
        assert_eq!(k, ["ESC", "F14", "NUPHY(0x06)", "TRNS", "MO(1)", "LIGHT(0x2a)", "MUTE"]);
    }

    #[test]
    fn parse_names_and_aliases() {
        assert_eq!(KeyCode::parse("KC_A").unwrap(), KeyCode(0x04));
        assert_eq!(KeyCode::parse("esc").unwrap(), KeyCode(0x29));
        assert_eq!(KeyCode::parse("Escape").unwrap(), KeyCode(0x29));
        assert_eq!(KeyCode::parse("trns").unwrap(), KC_TRNS);
        assert_eq!(KeyCode::parse("MO(1)").unwrap(), KeyCode(0x5221));
        assert_eq!(KeyCode::parse("tg(3)").unwrap(), KeyCode(0x5263));
        assert_eq!(KeyCode::parse("0x7e06").unwrap(), KeyCode(0x7E06));
        assert!(KeyCode::parse("MO(40)").is_err());
        assert!(KeyCode::parse("bogus").is_err());
    }

    #[test]
    fn modifier_wrappers_roundtrip() {
        let k = KeyCode::parse("LCTL(C)").unwrap();
        assert_eq!(k, KeyCode(0x0106));
        assert_eq!(k.to_string(), "LC(C)");
        let k = KeyCode::parse("LCTL(LSFT(ESC))").unwrap();
        assert_eq!(k, KeyCode(0x0329));
        assert_eq!(KeyCode::parse("RALT(A)").unwrap(), KeyCode(0x1404));
        assert_eq!(KeyCode::parse("RALT(A)").unwrap().to_string(), "RA(A)");
    }

    #[test]
    fn display_roundtrips_for_basic_names() {
        for (n, c) in BASIC {
            assert_eq!(KeyCode::parse(n).unwrap(), KeyCode(u16::from(*c)), "{n}");
            assert_eq!(KeyCode(u16::from(*c)).to_string(), *n);
        }
    }
}
