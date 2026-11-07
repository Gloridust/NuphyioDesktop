//! Linux `hidraw` backend for [`nuphy_proto::Transport`].
//!
//! Devices are found through sysfs (no libudev needed) and opened directly as
//! `/dev/hidrawN`. Other platforms get their own backend later (hidapi).

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::time::Duration;

use nuphy_proto::frame::{Frame, REPORT_LEN};
use nuphy_proto::{Transport, NUPHY_VID};

/// One `/dev/hidrawN` node belonging to a NuPhy device.
#[derive(Debug, Clone)]
pub struct HidrawInfo {
    pub path: PathBuf,
    pub vid: u16,
    pub pid: u16,
    pub name: String,
    /// USB interface number, when it can be derived from sysfs.
    pub interface: Option<u8>,
    /// Looks like the vendor configuration interface: top-level collection
    /// Usage Page 0x01 / Usage 0x00 with 64-byte reports (matches the web
    /// app's device filter).
    pub is_config: bool,
}

const CONFIG_DESCRIPTOR_PREFIX: [u8; 7] = [0x06, 0x01, 0x00, 0x09, 0x00, 0xA1, 0x01];

pub fn is_config_descriptor(desc: &[u8]) -> bool {
    desc.starts_with(&CONFIG_DESCRIPTOR_PREFIX) && desc.windows(2).any(|w| w == [0x95, 0x40])
}

fn parse_hid_id(uevent: &str) -> Option<(u16, u16)> {
    let id = uevent.lines().find_map(|l| l.strip_prefix("HID_ID="))?;
    let mut it = id.split(':');
    it.next()?; // bus
    let vid = u32::from_str_radix(it.next()?, 16).ok()?;
    let pid = u32::from_str_radix(it.next()?, 16).ok()?;
    Some((vid as u16, pid as u16))
}

fn interface_from_link(link: &Path) -> Option<u8> {
    // .../1-3/1-3:1.3/0003:19F5:1030.0019  → parent component "1-3:1.3"
    let parent = link.parent()?.file_name()?.to_str()?;
    parent.rsplit('.').next()?.parse().ok()
}

/// List hidraw nodes whose vendor id is NuPhy's.
pub fn enumerate() -> io::Result<Vec<HidrawInfo>> {
    enumerate_in(Path::new("/sys/class/hidraw"), Path::new("/dev"))
}

pub fn enumerate_in(sys: &Path, dev: &Path) -> io::Result<Vec<HidrawInfo>> {
    let mut out = Vec::new();
    for entry in fs::read_dir(sys)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let devdir = entry.path().join("device");
        let Ok(uevent) = fs::read_to_string(devdir.join("uevent")) else { continue };
        let Some((vid, pid)) = parse_hid_id(&uevent) else { continue };
        if vid != NUPHY_VID {
            continue;
        }
        let hid_name = uevent.lines().find_map(|l| l.strip_prefix("HID_NAME=")).unwrap_or("").to_string();
        let desc = fs::read(devdir.join("report_descriptor")).unwrap_or_default();
        let interface = fs::canonicalize(&devdir).ok().and_then(|p| interface_from_link(&p));
        out.push(HidrawInfo {
            path: dev.join(&name),
            vid,
            pid,
            name: hid_name,
            interface,
            is_config: is_config_descriptor(&desc),
        });
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

/// An open hidraw node.
pub struct Hidraw {
    file: File,
}

impl Hidraw {
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new().read(true).write(true).open(path)?;
        Ok(Self { file })
    }
}

impl Transport for Hidraw {
    fn write(&mut self, report: &Frame) -> io::Result<()> {
        // The config interface has no report ids, so the 64 bytes go out as-is.
        let n = self.file.write(report)?;
        if n != REPORT_LEN {
            return Err(io::Error::new(io::ErrorKind::WriteZero, format!("short write: {n}")));
        }
        Ok(())
    }

    fn read(&mut self, timeout: Duration) -> io::Result<Option<Frame>> {
        let mut pfd = libc::pollfd { fd: self.file.as_raw_fd(), events: libc::POLLIN, revents: 0 };
        let ms = timeout.as_millis().min(i32::MAX as u128) as i32;
        let rc = unsafe { libc::poll(&mut pfd, 1, ms) };
        if rc < 0 {
            return Err(io::Error::last_os_error());
        }
        if rc == 0 {
            return Ok(None);
        }
        let mut buf = [0u8; REPORT_LEN];
        let n = self.file.read(&mut buf)?;
        if n == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "device closed"));
        }
        Ok(Some(buf))
    }
}

/// Fill `N` bytes from the kernel CSPRNG.
pub fn random_bytes<const N: usize>() -> io::Result<[u8; N]> {
    let mut b = [0u8; N];
    File::open("/dev/urandom")?.read_exact(&mut b)?;
    Ok(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_descriptor_detection() {
        // hidraw6 of a Node 75 LP (interface 3), captured from sysfs.
        let cfg = hex("0601000900a1010902150026ff009540750881020902150026ff00954075089102c0");
        assert!(is_config_descriptor(&cfg));
        // hidraw2: the boot keyboard interface.
        let kbd = hex("05010906a101050719e029e7150025017501950881029501750881019503750105081901290391029505750191019506750826ff000507190029918100c0");
        assert!(!is_config_descriptor(&kbd));
    }

    #[test]
    fn hid_id_parsing() {
        assert_eq!(parse_hid_id("DRIVER=hid-generic\nHID_ID=0003:000019F5:00001030\n"), Some((0x19F5, 0x1030)));
        assert_eq!(parse_hid_id("nothing"), None);
    }

    #[test]
    fn interface_number() {
        let p = Path::new("/sys/devices/pci/usb1/1-3/1-3:1.3/0003:19F5:1030.0019");
        assert_eq!(interface_from_link(p), Some(3));
    }

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
}
