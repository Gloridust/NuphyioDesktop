use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::{Args, Parser, Subcommand};
use nuphy_hid::{enumerate, random_bytes, Hidraw, HidrawInfo};
use nuphy_proto::backup::Backup;
use nuphy_proto::cmd::mech;
use nuphy_proto::keycode::parse_codes;
use nuphy_proto::session::format_firmware_version;
use nuphy_proto::{device, BaseInfo, Family, KeyCode, Session};

#[derive(Parser)]
#[command(name = "nuphy", about = "Configure NuPhy keyboards offline")]
struct Cli {
    /// hidraw node; defaults to the first NuPhy config interface found.
    #[arg(long, global = true)]
    path: Option<PathBuf>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Args, Clone, Copy)]
struct KeyPos {
    /// Layer (0-based). Defaults to the active mode's base layer.
    #[arg(long)]
    layer: Option<u8>,
    /// Matrix row (with --col).
    #[arg(long, requires = "col", conflicts_with = "slot")]
    row: Option<u8>,
    /// Matrix column (with --row).
    #[arg(long, requires = "row")]
    col: Option<u8>,
    /// Raw slot index (row * cols + col; extra keys come after the matrix).
    #[arg(long)]
    slot: Option<usize>,
}

#[derive(Subcommand)]
enum Cmd {
    /// List NuPhy hidraw nodes and mark the configuration interface.
    List,
    /// Firmware version, modes, layers and matrix size.
    Info,
    /// Print a layer of the active key matrix as a grid.
    Keys {
        #[arg(long)]
        layer: Option<u8>,
        /// Print every layer.
        #[arg(long, conflicts_with = "layer")]
        all: bool,
        /// Show the factory-default matrix instead.
        #[arg(long)]
        default: bool,
    },
    /// Show one key.
    Get(KeyPos),
    /// Remap one key, e.g. `nuphy set --row 3 --col 0 esc` (CapsLock → Esc).
    ///
    /// Key names follow QMK: `a`, `esc`, `lctl`, `mo(1)`, `lctl(c)`, `trns`, or raw hex `0x7e06`.
    /// The whole matrix is backed up before writing.
    Set {
        #[command(flatten)]
        pos: KeyPos,
        key: String,
    },
    /// Put one key back to its factory default.
    Reset(KeyPos),
    /// Save the whole key matrix to a file.
    Backup { file: Option<PathBuf> },
    /// Write a key matrix saved with `backup` back to the keyboard.
    Restore { file: PathBuf },
    /// Read a raw region of a read-only command (hex), for exploration.
    Raw {
        /// Command id, e.g. a1 or 0xa1.
        cmd: String,
        #[arg(long, default_value_t = 0)]
        handle: u8,
        #[arg(long, default_value_t = 0)]
        addr: u16,
        #[arg(long, default_value_t = 64)]
        len: usize,
    },
}

struct Kb {
    s: Session<Hidraw>,
    info: HidrawInfo,
    base: BaseInfo,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), String> {
    let path = cli.path;
    match cli.cmd {
        Cmd::List => list(),
        Cmd::Info => {
            let mut kb = open(path)?;
            let fw = kb.s.read_region(mech::GET_FIRMWARE_INFO, 0, 8).map_err(e2s)?;
            let b = kb.base;
            println!("device     : {} ({:04x}:{:04x})", kb.info.name, kb.info.vid, kb.info.pid);
            println!("firmware   : {}", format_firmware_version(&fw).unwrap_or_else(|| "?".into()));
            println!("mode       : {} (layers {}..{} active)", b.current_mode, b.active_base_layer(),
                b.active_base_layer() + b.layers_per_mode - 1);
            println!("layers     : {} total, {} per mode", b.layer_count, b.layers_per_mode);
            println!("matrix     : {} rows x {} cols + {} extra slots", b.rows, b.cols, b.extra_keys);
            Ok(())
        }
        Cmd::Keys { layer, all, default } => {
            let mut kb = open(path)?;
            let b = kb.base;
            let cmd = if default { mech::GET_DEFAULT_KEYS } else { mech::GET_USE_KEYS };
            let raw = kb.s.read_region(cmd, 0, b.total_key_bytes()).map_err(e2s)?;
            let layers: Vec<u8> = if all { (0..b.layer_count).collect() } else { vec![layer.unwrap_or(b.active_base_layer())] };
            for l in layers {
                if l >= b.layer_count {
                    return Err(format!("layer {l} out of range (0..{})", b.layer_count));
                }
                let off = usize::from(l) * b.layer_bytes();
                print_layer(&b, l, &parse_codes(&raw[off..off + b.layer_bytes()]));
            }
            Ok(())
        }
        Cmd::Get(pos) => {
            let mut kb = open(path)?;
            let (l, slot) = resolve(&kb.base, pos)?;
            let cur = kb.s.get_key(&kb.base, l, slot).map_err(e2s)?;
            let def = kb.s.default_key(&kb.base, l, slot).map_err(e2s)?;
            println!("layer {l} slot {slot} ({}): {cur}  [{:#06x}]  default {def}", describe_slot(&kb.base, slot), cur.0);
            Ok(())
        }
        Cmd::Set { pos, key } => {
            let code = KeyCode::parse(&key)?;
            let mut kb = open(path)?;
            write_key(&mut kb, pos, Some(code))
        }
        Cmd::Reset(pos) => {
            let mut kb = open(path)?;
            write_key(&mut kb, pos, None)
        }
        Cmd::Backup { file } => {
            let mut kb = open(path)?;
            let p = save_backup(&mut kb, file)?;
            println!("saved {}", p.display());
            Ok(())
        }
        Cmd::Restore { file } => {
            let bytes = std::fs::read(&file).map_err(|e| format!("{}: {e}", file.display()))?;
            let bk = Backup::from_bytes(&bytes)?;
            let mut kb = open(path)?;
            bk.check_compatible(kb.info.vid, kb.info.pid, &kb.base)?;
            let safety = save_backup(&mut kb, None)?;
            println!("current matrix saved to {}", safety.display());
            kb.s.write_keymap(&kb.base, &bk.data).map_err(e2s)?;
            let now = kb.s.read_keymap(&kb.base).map_err(e2s)?;
            if now != bk.data {
                return Err(format!("verify failed after restore; previous state is in {}", safety.display()));
            }
            println!("restored {} bytes from {} (verified)", bk.data.len(), file.display());
            Ok(())
        }
        Cmd::Raw { cmd, handle, addr, len } => {
            let id = u8::from_str_radix(cmd.trim_start_matches("0x"), 16).map_err(|e| e.to_string())?;
            if !mech::READ_ONLY.contains(&id) {
                return Err(format!("{id:#04x} is not a known read-only command; refusing"));
            }
            let mut kb = open(path)?;
            let raw = kb.s.read_at(id, handle, addr, len).map_err(e2s)?;
            println!("{}", hex(&raw));
            Ok(())
        }
    }
}

fn write_key(kb: &mut Kb, pos: KeyPos, code: Option<KeyCode>) -> Result<(), String> {
    let (l, slot) = resolve(&kb.base, pos)?;
    let old = kb.s.get_key(&kb.base, l, slot).map_err(e2s)?;
    let new = match code {
        Some(c) => c,
        None => kb.s.default_key(&kb.base, l, slot).map_err(e2s)?,
    };
    if old == new {
        println!("layer {l} slot {slot}: already {new}, nothing to do");
        return Ok(());
    }
    let bk = save_backup(kb, None)?;
    kb.s.set_key(&kb.base, l, slot, new).map_err(e2s)?;
    let now = kb.s.get_key(&kb.base, l, slot).map_err(e2s)?;
    if now != new {
        return Err(format!("verify failed: wrote {new}, read back {now}. Backup: {}", bk.display()));
    }
    println!("layer {l} slot {slot} ({}): {old} -> {new}  (backup {})", describe_slot(&kb.base, slot), bk.display());
    Ok(())
}

fn resolve(b: &BaseInfo, pos: KeyPos) -> Result<(u8, usize), String> {
    let layer = pos.layer.unwrap_or(b.active_base_layer());
    if layer >= b.layer_count {
        return Err(format!("layer {layer} out of range (0..{})", b.layer_count));
    }
    let slot = match (pos.slot, pos.row, pos.col) {
        (Some(s), _, _) => s,
        (None, Some(r), Some(c)) => {
            if r >= b.rows || c >= b.cols {
                return Err(format!("row/col out of range ({} x {})", b.rows, b.cols));
            }
            usize::from(r) * usize::from(b.cols) + usize::from(c)
        }
        _ => return Err("give --row and --col, or --slot".into()),
    };
    if slot >= b.slots_per_layer() {
        return Err(format!("slot {slot} out of range (0..{})", b.slots_per_layer()));
    }
    Ok((layer, slot))
}

fn describe_slot(b: &BaseInfo, slot: usize) -> String {
    let m = usize::from(b.rows) * usize::from(b.cols);
    if slot < m {
        format!("r{} c{}", slot / usize::from(b.cols), slot % usize::from(b.cols))
    } else {
        format!("extra {}", slot - m)
    }
}

fn print_layer(b: &BaseInfo, layer: u8, codes: &[KeyCode]) {
    let mode = layer / b.layers_per_mode.max(1);
    let active = if mode == b.current_mode { " (active mode)" } else { "" };
    println!("== layer {layer}  mode {mode}{active}");
    let cols = usize::from(b.cols);
    print!("     ");
    for c in 0..cols {
        print!("{:<9}", format!("c{c}"));
    }
    println!();
    for (r, row) in codes[..usize::from(b.rows) * cols].chunks(cols).enumerate() {
        print!("r{r:<3} ");
        for k in row {
            print!("{:<9}", label(*k));
        }
        println!();
    }
    let extra: Vec<String> = codes[usize::from(b.rows) * cols..].iter().map(|k| label(*k)).collect();
    if !extra.is_empty() {
        println!("extra {}", extra.join("  "));
    }
    println!();
}

fn label(k: KeyCode) -> String {
    match k.0 {
        0x0000 => ".".into(),
        0x0001 => "_".into(),
        _ => {
            let s = k.to_string();
            if s.len() > 8 { format!("{:#06x}", k.0) } else { s }
        }
    }
}

fn backup_dir() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("nuphy/backups")
}

fn save_backup(kb: &mut Kb, file: Option<PathBuf>) -> Result<PathBuf, String> {
    let data = kb.s.read_keymap(&kb.base).map_err(e2s)?;
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

fn list() -> Result<(), String> {
    let nodes = enumerate().map_err(|e| format!("cannot read /sys/class/hidraw: {e}"))?;
    if nodes.is_empty() {
        println!("no NuPhy hidraw nodes found");
        return Ok(());
    }
    for n in nodes {
        let d = device::lookup(n.vid, n.pid);
        let family = match d.map(|d| d.family) {
            Some(Family::Mechanical) => "mechanical",
            Some(Family::Hall) => "hall",
            _ => "?",
        };
        println!(
            "{}  {:04x}:{:04x}  if{}  {}  [{}, {}]{}",
            n.path.display(),
            n.vid,
            n.pid,
            n.interface.map_or("?".into(), |i| i.to_string()),
            n.name,
            d.map(|d| d.name).unwrap_or("unknown model"),
            family,
            if n.is_config { "  <- config interface" } else { "" }
        );
    }
    Ok(())
}

fn open(path: Option<PathBuf>) -> Result<Kb, String> {
    let nodes = enumerate().map_err(|e| e.to_string())?;
    let info = match path {
        Some(p) => nodes.into_iter().find(|n| n.path == p).ok_or_else(|| format!("{} is not a NuPhy hidraw node", p.display()))?,
        None => nodes
            .into_iter()
            .find(|n| n.is_config)
            .ok_or("no NuPhy configuration interface found (try `nuphy list`)")?,
    };
    match device::lookup(info.vid, info.pid) {
        Some(d) if d.family == Family::Mechanical => {}
        Some(d) => {
            return Err(format!("{} uses the {:?} protocol family; only the mechanical family is implemented", d.name, d.family))
        }
        None => return Err(format!("unknown model {:04x}:{:04x}", info.vid, info.pid)),
    }
    let t = Hidraw::open(&info.path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            format!(
                "cannot open {}: permission denied.\nInstall contrib/udev/70-nuphy.rules (see docs/PROTOCOL.md §10).",
                info.path.display()
            )
        } else {
            format!("cannot open {}: {e}", info.path.display())
        }
    })?;
    let rnd = random_bytes::<56>().map_err(|e| e.to_string())?;
    let mut s = Session::open(t, rnd).map_err(e2s)?;
    let base = s.base_info().map_err(e2s)?;
    if base.layer_count == 0 || base.rows == 0 || base.cols == 0 {
        return Err(format!("keyboard reported an empty matrix: {base:?}"));
    }
    Ok(Kb { s, info, base })
}

fn e2s(e: nuphy_proto::SessionError) -> String {
    e.to_string()
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect::<Vec<_>>().join(" ")
}
