//! Key-matrix backup file: a small header + the raw `GetUseKeys` bytes.
//!
//! ```text
//! 0  8  magic "NUPHYKM1"
//! 8  2  vid (LE)
//! 10 2  pid (LE)
//! 12 2  bytes per layer (LE)
//! 14 1  layer count
//! 15 1  reserved (0)
//! 16 .. matrix (layer_count * bytes_per_layer)
//! ```

use crate::base::BaseInfo;

const MAGIC: &[u8; 8] = b"NUPHYKM1";
const HEADER: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Backup {
    pub vid: u16,
    pub pid: u16,
    pub layer_bytes: u16,
    pub layer_count: u8,
    pub data: Vec<u8>,
}

impl Backup {
    pub fn new(vid: u16, pid: u16, base: &BaseInfo, data: Vec<u8>) -> Self {
        Self { vid, pid, layer_bytes: base.layer_bytes() as u16, layer_count: base.layer_count, data }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(HEADER + self.data.len());
        v.extend_from_slice(MAGIC);
        v.extend_from_slice(&self.vid.to_le_bytes());
        v.extend_from_slice(&self.pid.to_le_bytes());
        v.extend_from_slice(&self.layer_bytes.to_le_bytes());
        v.push(self.layer_count);
        v.push(0);
        v.extend_from_slice(&self.data);
        v
    }

    pub fn from_bytes(b: &[u8]) -> Result<Self, String> {
        if b.len() < HEADER || &b[..8] != MAGIC {
            return Err("not a NuPhy key-matrix backup".into());
        }
        let u16le = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
        let bk = Self {
            vid: u16le(8),
            pid: u16le(10),
            layer_bytes: u16le(12),
            layer_count: b[14],
            data: b[HEADER..].to_vec(),
        };
        if bk.data.len() != usize::from(bk.layer_bytes) * usize::from(bk.layer_count) {
            return Err(format!(
                "backup is truncated: {} bytes, expected {}",
                bk.data.len(),
                usize::from(bk.layer_bytes) * usize::from(bk.layer_count)
            ));
        }
        Ok(bk)
    }

    /// Whether this backup can be written to a keyboard with `base` geometry.
    pub fn check_compatible(&self, vid: u16, pid: u16, base: &BaseInfo) -> Result<(), String> {
        if (self.vid, self.pid) != (vid, pid) {
            return Err(format!("backup is for {:04x}:{:04x}, keyboard is {vid:04x}:{pid:04x}", self.vid, self.pid));
        }
        if usize::from(self.layer_bytes) != base.layer_bytes() || self.layer_count != base.layer_count {
            return Err("backup geometry differs from the keyboard (firmware changed?)".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> BaseInfo {
        BaseInfo::parse(&[1, 8, 4, 6, 16, 0x85], 0).unwrap()
    }

    #[test]
    fn roundtrip_and_checks() {
        let b = Backup::new(0x19F5, 0x1030, &base(), vec![7; 1616]);
        let bytes = b.to_bytes();
        assert_eq!(&bytes[..8], b"NUPHYKM1");
        let back = Backup::from_bytes(&bytes).unwrap();
        assert_eq!(back, b);
        assert!(back.check_compatible(0x19F5, 0x1030, &base()).is_ok());
        assert!(back.check_compatible(0x19F5, 0x1031, &base()).is_err());
        assert!(Backup::from_bytes(&bytes[..100]).is_err());
        assert!(Backup::from_bytes(b"garbage garbage garbage").is_err());
    }
}
