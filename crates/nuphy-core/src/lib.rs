//! Things the CLI and the GUI both need: find + open the keyboard, and keep
//! automatic backups of the key matrix.

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use nuphy_hid::{enumerate, random_bytes, Hidraw, HidrawInfo};
use nuphy_proto::backup::Backup;
use nuphy_proto::{device, BaseInfo, Family, Session, SessionError};

/// An open keyboard: session, hidraw node info and the key-matrix geometry.
pub struct Kb {
    pub s: Session<Hidraw>,
    pub info: HidrawInfo,
    pub base: BaseInfo,
}

#[derive(Debug)]
pub enum OpenError {
    NoDevice,
    NotNuphy(PathBuf),
    Permission(PathBuf),
    UnsupportedFamily(String),
    UnknownModel(u16, u16),
    EmptyMatrix,
    Other(String),
}

impl OpenError {
    /// Stable identifier, so a UI can translate the message.
    pub fn code(&self) -> &'static str {
        match self {
            Self::NoDevice => "no_device",
            Self::NotNuphy(_) => "not_nuphy",
            Self::Permission(_) => "permission",
            Self::UnsupportedFamily(_) => "unsupported_family",
            Self::UnknownModel(..) => "unknown_model",
            Self::EmptyMatrix => "empty_matrix",
            Self::Other(_) => "other",
        }
    }
}

impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoDevice => write!(f, "no NuPhy configuration interface found (try `nuphy list`)"),
            Self::NotNuphy(p) => write!(f, "{} is not a NuPhy hidraw node", p.display()),
            Self::Permission(p) => write!(
                f,
                "cannot open {}: permission denied.\nInstall contrib/udev/70-nuphy.rules (see docs/PROTOCOL.md §10).",
                p.display()
            ),
            Self::UnsupportedFamily(n) => {
                write!(f, "{n} uses a protocol family that is not implemented (only the mechanical family is)")
            }
            Self::UnknownModel(v, p) => write!(f, "unknown model {v:04x}:{p:04x}"),
            Self::EmptyMatrix => write!(f, "keyboard reported an empty key matrix"),
            Self::Other(s) => f.write_str(s),
        }
    }
}

impl std::error::Error for OpenError {}

impl From<SessionError> for OpenError {
    fn from(e: SessionError) -> Self {
        Self::Other(e.to_string())
    }
}

/// Open `path`, or the first NuPhy configuration interface when `None`.
pub fn open(path: Option<&Path>) -> Result<Kb, OpenError> {
    let nodes = enumerate().map_err(|e| OpenError::Other(e.to_string()))?;
    let info = match path {
        Some(p) => nodes.into_iter().find(|n| n.path == p).ok_or_else(|| OpenError::NotNuphy(p.to_path_buf()))?,
        None => nodes.into_iter().find(|n| n.is_config).ok_or(OpenError::NoDevice)?,
    };
    match device::lookup(info.vid, info.pid) {
        Some(d) if d.family == Family::Mechanical => {}
        Some(d) => return Err(OpenError::UnsupportedFamily(d.name.to_string())),
        None => return Err(OpenError::UnknownModel(info.vid, info.pid)),
    }
    let t = Hidraw::open(&info.path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            OpenError::Permission(info.path.clone())
        } else {
            OpenError::Other(format!("cannot open {}: {e}", info.path.display()))
        }
    })?;
    let rnd = random_bytes::<56>().map_err(|e| OpenError::Other(e.to_string()))?;
    let mut s = Session::open(t, rnd)?;
    let base = s.base_info()?;
    if base.layer_count == 0 || base.rows == 0 || base.cols == 0 {
        return Err(OpenError::EmptyMatrix);
    }
    Ok(Kb { s, info, base })
}

/// `$XDG_DATA_HOME/nuphy/backups` (default `~/.local/share/nuphy/backups`).
pub fn backup_dir() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("nuphy/backups")
}

/// Save the whole key matrix. `None` → a timestamped file in [`backup_dir`].
pub fn save_backup(kb: &mut Kb, file: Option<PathBuf>) -> Result<PathBuf, String> {
    let data = kb.s.read_keymap(&kb.base).map_err(|e| e.to_string())?;
    let bk = Backup::new(kb.info.vid, kb.info.pid, &kb.base, data);
    let path = match file {
        Some(p) => p,
        None => {
            let dir = backup_dir();
            std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            let ts = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
            dir.join(format!("{:04x}-{ts}.bin", kb.info.pid))
        }
    };
    std::fs::write(&path, bk.to_bytes()).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

/// Write a backup file back to the keyboard (current state is saved first) and verify.
/// Returns the path of the safety backup.
pub fn restore_backup(kb: &mut Kb, file: &Path) -> Result<PathBuf, String> {
    let bytes = std::fs::read(file).map_err(|e| format!("{}: {e}", file.display()))?;
    let bk = Backup::from_bytes(&bytes)?;
    bk.check_compatible(kb.info.vid, kb.info.pid, &kb.base)?;
    let safety = save_backup(kb, None)?;
    kb.s.write_keymap(&kb.base, &bk.data).map_err(|e| e.to_string())?;
    let now = kb.s.read_keymap(&kb.base).map_err(|e| e.to_string())?;
    if now != bk.data {
        return Err(format!("verify failed after restore; previous state is in {}", safety.display()));
    }
    Ok(safety)
}

/// Backup files in [`backup_dir`] as `(path, unix_ms)`, newest first.
pub fn list_backups() -> Vec<(PathBuf, u128)> {
    let mut v: Vec<(PathBuf, u128)> = std::fs::read_dir(backup_dir())
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            let stem = p.file_stem()?.to_str()?;
            let ms = stem.split_once('-')?.1.parse::<u128>().ok()?;
            (p.extension()? == "bin").then_some((p, ms))
        })
        .collect();
    v.sort_by_key(|a| std::cmp::Reverse(a.1));
    v
}
