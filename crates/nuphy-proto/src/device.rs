//! Known NuPhy devices (from the NuPhyIO web app's device table).

use crate::devices_table::DEVICES;

pub const NUPHY_VID: u16 = 0x19F5;

/// Which of the two wire protocols a model speaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    /// Mechanical boards: secret-key handshake + XOR (`cmd::mech`).
    Mechanical,
    /// Hall-effect boards (`cmd::hall`).
    Hall,
    /// The web app's table doesn't say; needs verifying on hardware.
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Normal,
    /// IAP bootloader (firmware update mode).
    Bootloader,
}

#[derive(Clone, Copy, Debug)]
pub struct DeviceInfo {
    pub vid: u16,
    pub pid: u16,
    pub name: &'static str,
    pub mode: Mode,
    /// Inferred from the model name (`…HE` ⇒ Hall); not read from hardware.
    pub family: Family,
}

pub fn lookup(vid: u16, pid: u16) -> Option<&'static DeviceInfo> {
    DEVICES.iter().find(|d| d.vid == vid && d.pid == pid)
}

pub fn all() -> &'static [DeviceInfo] {
    DEVICES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node75_is_known_and_mechanical() {
        let d = lookup(NUPHY_VID, 0x1030).unwrap();
        assert_eq!(d.name, "Node75");
        assert_eq!(d.family, Family::Mechanical);
        assert_eq!(d.mode, Mode::Normal);
    }

    #[test]
    fn he_models_are_hall_and_pids_unique() {
        assert_eq!(lookup(NUPHY_VID, 0x6120).unwrap().family, Family::Hall);
        let mut seen = std::collections::HashSet::new();
        for d in all() {
            assert!(seen.insert((d.vid, d.pid)), "duplicate {:04x}", d.pid);
        }
    }
}
