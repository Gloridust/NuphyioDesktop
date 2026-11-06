//! Key-matrix entries (`GetDefaultKeys` / `GetUseKeys`).
//!
//! Each key slot is two bytes `[kind, code]`. `kind == 0` means a plain USB
//! HID keyboard usage (e.g. `[0, 0x29]` = Esc). Other kinds (`0x7E`, `0x52`,
//! `0x78`, …) are NuPhy-specific actions whose meaning is not yet decoded.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEntry {
    pub kind: u8,
    pub code: u8,
}

impl KeyEntry {
    pub fn is_plain_hid(&self) -> bool {
        self.kind == 0
    }

    pub fn is_empty(&self) -> bool {
        self.kind == 0 && self.code == 0
    }

    /// Human-readable label; falls back to hex for undecoded kinds.
    pub fn label(&self) -> String {
        if self.is_plain_hid() {
            match hid_usage_name(self.code) {
                Some(n) => n.to_string(),
                None => format!("HID {:#04x}", self.code),
            }
        } else {
            format!("[{:#04x} {:#04x}]", self.kind, self.code)
        }
    }
}

/// Split a raw region into key entries (a trailing odd byte is ignored).
pub fn parse_entries(raw: &[u8]) -> Vec<KeyEntry> {
    raw.as_chunks::<2>().0.iter().map(|c| KeyEntry { kind: c[0], code: c[1] }).collect()
}

/// USB HID keyboard-page usage names for the common keys.
pub fn hid_usage_name(code: u8) -> Option<&'static str> {
    const DIGITS: &[&str] = &["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"];
    const LETTER_NAMES: [&str; 26] = [
        "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S",
        "T", "U", "V", "W", "X", "Y", "Z",
    ];
    Some(match code {
        0x04..=0x1D => LETTER_NAMES[(code - 0x04) as usize],
        0x1E..=0x27 => DIGITS[(code - 0x1E) as usize],
        0x28 => "Enter",
        0x29 => "Esc",
        0x2A => "Backspace",
        0x2B => "Tab",
        0x2C => "Space",
        0x2D => "-",
        0x2E => "=",
        0x2F => "[",
        0x30 => "]",
        0x31 => "\\",
        0x33 => ";",
        0x34 => "'",
        0x35 => "`",
        0x36 => ",",
        0x37 => ".",
        0x38 => "/",
        0x39 => "CapsLock",
        0x3A..=0x45 => ["F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12"]
            [(code - 0x3A) as usize],
        0x46 => "PrintScreen",
        0x47 => "ScrollLock",
        0x48 => "Pause",
        0x49 => "Insert",
        0x4A => "Home",
        0x4B => "PageUp",
        0x4C => "Delete",
        0x4D => "End",
        0x4E => "PageDown",
        0x4F => "Right",
        0x50 => "Left",
        0x51 => "Down",
        0x52 => "Up",
        0xE0 => "LCtrl",
        0xE1 => "LShift",
        0xE2 => "LAlt",
        0xE3 => "LGui",
        0xE4 => "RCtrl",
        0xE5 => "RShift",
        0xE6 => "RAlt",
        0xE7 => "RGui",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_start_of_the_web_apps_sample_matrix() {
        // First slots of the Node75-family sample in the NuPhyIO simulator.
        let raw = [0, 41, 0, 105, 0, 106, 126, 6, 0, 53, 0, 30, 0, 31];
        let e = parse_entries(&raw);
        assert_eq!(e[0].label(), "Esc");
        assert_eq!(e[1].label(), "HID 0x69"); // F14, not in the name table
        assert_eq!(e[3].label(), "[0x7e 0x06]");
        assert_eq!(e[4].label(), "`");
        assert_eq!(e[5].label(), "1");
        assert_eq!(e[6].label(), "2");
    }

    #[test]
    fn odd_trailing_byte_ignored() {
        assert_eq!(parse_entries(&[0, 4, 9]).len(), 1);
    }
}
