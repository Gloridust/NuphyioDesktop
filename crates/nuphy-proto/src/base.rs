//! `GetBase` (0xA0) block: modes, layers and matrix geometry.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BaseInfo {
    /// Active mode (e.g. Mac / Windows). Each mode owns `layers_per_mode` layers.
    pub current_mode: u8,
    pub layer_count: u8,
    pub layers_per_mode: u8,
    pub rows: u8,
    pub cols: u8,
    /// Key slots after the `rows × cols` matrix (knob etc.).
    pub extra_keys: u8,
}

impl BaseInfo {
    /// Parse the first 6 bytes of `GetBase`. `session_key` is needed because
    /// the firmware leaves unused bytes un-encrypted: after decryption a zero
    /// byte shows up as the key itself.
    pub fn parse(d: &[u8], session_key: u8) -> Option<Self> {
        if d.len() < 6 {
            return None;
        }
        let raw = if d[5] == session_key { 0 } else { d[5] };
        let extra_keys = if raw & 0x80 != 0 { raw & 0x7F } else { 2 };
        Some(Self { current_mode: d[0], layer_count: d[1], layers_per_mode: d[2], rows: d[3], cols: d[4], extra_keys })
    }

    pub fn slots_per_layer(&self) -> usize {
        usize::from(self.rows) * usize::from(self.cols) + usize::from(self.extra_keys)
    }

    /// Bytes of key matrix per layer (`keysDataSize` in the web app).
    pub fn layer_bytes(&self) -> usize {
        self.slots_per_layer() * 2
    }

    pub fn total_key_bytes(&self) -> usize {
        self.layer_bytes() * usize::from(self.layer_count)
    }

    /// Matrix address of a key slot.
    pub fn key_addr(&self, layer: u8, slot: usize) -> Option<u16> {
        if layer >= self.layer_count || slot >= self.slots_per_layer() {
            return None;
        }
        u16::try_from(usize::from(layer) * self.layer_bytes() + slot * 2).ok()
    }

    /// First layer of the active mode.
    pub fn active_base_layer(&self) -> u8 {
        self.current_mode.saturating_mul(self.layers_per_mode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node75_lp_real_values() {
        // Read from a Node 75 LP (fw 1.0.10.6); session key happened to be 0x83.
        let d = [0x01, 0x08, 0x04, 0x06, 0x10, 0x85, 0x83, 0x83];
        let b = BaseInfo::parse(&d, 0x83).unwrap();
        assert_eq!((b.current_mode, b.layer_count, b.layers_per_mode), (1, 8, 4));
        assert_eq!((b.rows, b.cols, b.extra_keys), (6, 16, 5));
        assert_eq!(b.layer_bytes(), 202);
        assert_eq!(b.total_key_bytes(), 1616);
        assert_eq!(b.key_addr(4, 0), Some(808));
        assert_eq!(b.key_addr(0, 101), None);
        assert_eq!(b.active_base_layer(), 4);
    }

    #[test]
    fn unset_extra_byte_means_two() {
        let b = BaseInfo::parse(&[0, 2, 1, 6, 16, 0x5A], 0x5A).unwrap();
        assert_eq!(b.extra_keys, 2);
    }
}
