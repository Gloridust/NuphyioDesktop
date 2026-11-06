use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use nuphy_hid::{enumerate, random_bytes, Hidraw, HidrawInfo};
use nuphy_proto::cmd::mech;
use nuphy_proto::keymap::parse_entries;
use nuphy_proto::session::format_firmware_version;
use nuphy_proto::{device, Family, Session};

#[derive(Parser)]
#[command(name = "nuphy", about = "NuPhy keyboard tool (read-only for now)")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// List NuPhy hidraw nodes and mark the configuration interface.
    List,
    /// Handshake, then print firmware version and the base-info block.
    Info {
        /// hidraw node; defaults to the first config interface found.
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Dump the active key matrix (`GetUseKeys`).
    Keys {
        #[arg(long)]
        path: Option<PathBuf>,
        /// Bytes to read (2 per key slot; the web app's samples are ~1500).
        #[arg(long, default_value_t = 1568)]
        len: usize,
        /// Read the factory-default matrix instead of the active one.
        #[arg(long)]
        default: bool,
    },
    /// Read a raw region of a read-only command (hex), for exploration.
    Raw {
        /// Command id, e.g. a1 or 0xa1.
        cmd: String,
        #[arg(long, default_value_t = 0)]
        handle: u8,
        #[arg(long, default_value_t = 64)]
        len: usize,
        #[arg(long)]
        path: Option<PathBuf>,
    },
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
    match cli.cmd {
        Cmd::List => list(),
        Cmd::Info { path } => {
            let mut s = open(path)?;
            let fw = s.read_region(mech::GET_FIRMWARE_INFO, 0, 8).map_err(|e| e.to_string())?;
            let base = s.read_region(mech::GET_BASE, 0, 32).map_err(|e| e.to_string())?;
            println!("firmware raw : {}", hex(&fw));
            println!("firmware     : {}", format_firmware_version(&fw).unwrap_or_else(|| "?".into()));
            println!("base (32 B)  : {}", hex(&base));
            Ok(())
        }
        Cmd::Keys { path, len, default } => {
            let mut s = open(path)?;
            let cmd = if default { mech::GET_DEFAULT_KEYS } else { mech::GET_USE_KEYS };
            let raw = s.read_region(cmd, 0, len).map_err(|e| e.to_string())?;
            for (i, row) in parse_entries(&raw).chunks(8).enumerate() {
                let cells: Vec<String> = row.iter().map(|e| format!("{:<12}", e.label())).collect();
                println!("{:4}  {}", i * 8, cells.join(" "));
            }
            Ok(())
        }
        Cmd::Raw { cmd, handle, len, path } => {
            let id = u8::from_str_radix(cmd.trim_start_matches("0x"), 16).map_err(|e| e.to_string())?;
            if !mech::READ_ONLY.contains(&id) {
                return Err(format!("{id:#04x} is not a known read-only command; refusing"));
            }
            let mut s = open(path)?;
            let raw = s.read_region(id, handle, len).map_err(|e| e.to_string())?;
            println!("{}", hex(&raw));
            Ok(())
        }
    }
}

fn list() -> Result<(), String> {
    let nodes = enumerate().map_err(|e| format!("cannot read /sys/class/hidraw: {e}"))?;
    if nodes.is_empty() {
        println!("no NuPhy hidraw nodes found");
        return Ok(());
    }
    for n in nodes {
        let model = device::lookup(n.vid, n.pid).map(|d| d.name).unwrap_or("unknown model");
        let family = match device::lookup(n.vid, n.pid).map(|d| d.family) {
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
            model,
            family,
            if n.is_config { "  <- config interface" } else { "" }
        );
    }
    Ok(())
}

fn open(path: Option<PathBuf>) -> Result<Session<Hidraw>, String> {
    let info = match path {
        Some(p) => HidrawInfo {
            path: p,
            vid: 0,
            pid: 0,
            name: String::new(),
            interface: None,
            is_config: true,
        },
        None => enumerate()
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|n| n.is_config)
            .ok_or("no NuPhy configuration interface found (try `nuphy list`)")?,
    };
    if let Some(d) = device::lookup(info.vid, info.pid) {
        if d.family != Family::Mechanical {
            return Err(format!(
                "{} uses the {:?} protocol family; only the mechanical family is implemented",
                d.name, d.family
            ));
        }
    }
    let t = Hidraw::open(&info.path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            format!(
                "cannot open {}: permission denied.\nInstall contrib/udev/70-nuphy.rules (see docs/PROTOCOL.md §7).",
                info.path.display()
            )
        } else {
            format!("cannot open {}: {e}", info.path.display())
        }
    })?;
    let rnd = random_bytes::<56>().map_err(|e| e.to_string())?;
    Session::open(t, rnd).map_err(|e| e.to_string())
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect::<Vec<_>>().join(" ")
}
