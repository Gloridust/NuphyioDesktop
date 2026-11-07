#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! NuPhy Desktop: Tauri shell around `nuphy-core`. All keyboard access happens
//! here; the web frontend in `dist/` only calls the commands below.

use std::path::PathBuf;
use std::sync::Mutex;

use nuphy_core::{self as core, Kb, OpenError};
use nuphy_proto::cmd::mech;
use nuphy_proto::keycode::parse_codes;
use nuphy_proto::session::format_firmware_version;
use nuphy_proto::{device, KeyCode};
use serde::Serialize;
use tauri::State;

#[cfg(test)]
mod locale_tests;

#[derive(Default)]
struct App(Mutex<Option<Kb>>);

/// Error sent to the frontend: `code` is translated there, `detail` is shown as-is.
#[derive(Serialize)]
struct ApiError {
    code: String,
    detail: String,
}

impl ApiError {
    fn new(code: &str, detail: impl ToString) -> Self {
        Self { code: code.into(), detail: detail.to_string() }
    }
    fn other(detail: impl ToString) -> Self {
        Self::new("other", detail)
    }
}

impl From<OpenError> for ApiError {
    fn from(e: OpenError) -> Self {
        Self::new(e.code(), e)
    }
}

type Res<T> = Result<T, ApiError>;

#[derive(Serialize)]
struct KeyDto {
    code: u16,
    label: String,
}

impl From<KeyCode> for KeyDto {
    fn from(k: KeyCode) -> Self {
        Self { code: k.0, label: k.to_string() }
    }
}

#[derive(Serialize)]
struct StateDto {
    path: String,
    name: String,
    model: String,
    vid: u16,
    pid: u16,
    firmware: String,
    current_mode: u8,
    layer_count: u8,
    layers_per_mode: u8,
    rows: u8,
    cols: u8,
    extra: u8,
    active_layer: u8,
    slots_per_layer: usize,
    /// `layer_count * slots_per_layer` keys, layer-major.
    keys: Vec<KeyDto>,
    /// Factory defaults in the same order.
    defaults: Vec<u16>,
}

#[derive(Serialize)]
struct BackupDto {
    path: String,
    name: String,
    time_ms: u64,
}

fn snapshot(kb: &mut Kb) -> Res<StateDto> {
    let b = kb.base;
    let n = b.total_key_bytes();
    let fw = kb.s.read_region(mech::GET_FIRMWARE_INFO, 0, 8).map_err(ApiError::other)?;
    let cur = kb.s.read_region(mech::GET_USE_KEYS, 0, n).map_err(ApiError::other)?;
    let def = kb.s.read_region(mech::GET_DEFAULT_KEYS, 0, n).map_err(ApiError::other)?;
    Ok(StateDto {
        path: kb.info.path.display().to_string(),
        name: kb.info.name.clone(),
        model: device::lookup(kb.info.vid, kb.info.pid).map(|d| d.name.to_string()).unwrap_or_default(),
        vid: kb.info.vid,
        pid: kb.info.pid,
        firmware: format_firmware_version(&fw).unwrap_or_default(),
        current_mode: b.current_mode,
        layer_count: b.layer_count,
        layers_per_mode: b.layers_per_mode,
        rows: b.rows,
        cols: b.cols,
        extra: b.extra_keys,
        active_layer: b.active_base_layer(),
        slots_per_layer: b.slots_per_layer(),
        keys: parse_codes(&cur).into_iter().map(KeyDto::from).collect(),
        defaults: parse_codes(&def).into_iter().map(|k| k.0).collect(),
    })
}

fn with_kb<T>(app: &State<App>, f: impl FnOnce(&mut Kb) -> Res<T>) -> Res<T> {
    let mut g = app.0.lock().map_err(|_| ApiError::other("internal lock poisoned"))?;
    let kb = g.as_mut().ok_or_else(|| ApiError::new("not_connected", "not connected"))?;
    f(kb)
}

fn check_pos(kb: &Kb, layer: u8, slot: usize) -> Res<()> {
    if layer >= kb.base.layer_count || slot >= kb.base.slots_per_layer() {
        return Err(ApiError::new("out_of_range", format!("layer {layer} slot {slot}")));
    }
    Ok(())
}

/// Connect to the first NuPhy configuration interface.
#[tauri::command]
fn connect(app: State<App>) -> Res<StateDto> {
    let mut guard = app.0.lock().map_err(|_| ApiError::other("internal lock poisoned"))?;
    *guard = None; // close any previous handle first
    let mut kb = core::open(None)?;
    let st = snapshot(&mut kb)?;
    *guard = Some(kb);
    Ok(st)
}

#[tauri::command]
fn disconnect(app: State<App>) {
    if let Ok(mut g) = app.0.lock() {
        *g = None;
    }
}

#[tauri::command]
fn refresh(app: State<App>) -> Res<StateDto> {
    with_kb(&app, snapshot)
}

/// Parse key text without touching the keyboard (live preview in the editor).
#[tauri::command]
fn parse_key(text: String) -> Res<KeyDto> {
    KeyCode::parse(&text).map(KeyDto::from).map_err(|e| ApiError::new("bad_key", e))
}

/// Remap one key: back up the whole matrix, write, read back and verify.
#[tauri::command]
fn set_key(app: State<App>, layer: u8, slot: usize, text: String) -> Res<KeyDto> {
    let code = KeyCode::parse(&text).map_err(|e| ApiError::new("bad_key", e))?;
    with_kb(&app, |kb| {
        check_pos(kb, layer, slot)?;
        write_verified(kb, layer, slot, code)
    })
}

/// Put one key back to its factory value.
#[tauri::command]
fn reset_key(app: State<App>, layer: u8, slot: usize) -> Res<KeyDto> {
    with_kb(&app, |kb| {
        check_pos(kb, layer, slot)?;
        let def = kb.s.default_key(&kb.base, layer, slot).map_err(ApiError::other)?;
        write_verified(kb, layer, slot, def)
    })
}

fn write_verified(kb: &mut Kb, layer: u8, slot: usize, code: KeyCode) -> Res<KeyDto> {
    let old = kb.s.get_key(&kb.base, layer, slot).map_err(ApiError::other)?;
    if old == code {
        return Ok(code.into());
    }
    core::save_backup(kb, None).map_err(ApiError::other)?;
    kb.s.set_key(&kb.base, layer, slot, code).map_err(ApiError::other)?;
    let now = kb.s.get_key(&kb.base, layer, slot).map_err(ApiError::other)?;
    if now != code {
        return Err(ApiError::new("verify_failed", format!("wrote {code}, read back {now}")));
    }
    Ok(now.into())
}

#[tauri::command]
fn list_backups() -> Vec<BackupDto> {
    core::list_backups()
        .into_iter()
        .map(|(p, ms)| BackupDto {
            name: p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
            path: p.display().to_string(),
            time_ms: ms as u64,
        })
        .collect()
}

#[tauri::command]
fn make_backup(app: State<App>) -> Res<String> {
    with_kb(&app, |kb| core::save_backup(kb, None).map(|p| p.display().to_string()).map_err(ApiError::other))
}

/// Only files inside the backup directory may be restored.
#[tauri::command]
fn restore_backup(app: State<App>, path: String) -> Res<StateDto> {
    let p = PathBuf::from(&path);
    let inside = match (p.canonicalize(), core::backup_dir().canonicalize()) {
        (Ok(f), Ok(d)) => f.parent() == Some(d.as_path()),
        _ => false,
    };
    if !inside {
        return Err(ApiError::new("bad_backup", path));
    }
    with_kb(&app, |kb| {
        core::restore_backup(kb, &p).map_err(|e| ApiError::new("restore_failed", e))?;
        snapshot(kb)
    })
}

fn main() {
    // WebKitGTK's DMABUF renderer draws garbage (other windows' pixels) on some
    // Wayland + NVIDIA setups. Turn it off unless the user chose otherwise.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    tauri::Builder::default()
        .manage(App::default())
        .invoke_handler(tauri::generate_handler![
            connect,
            disconnect,
            refresh,
            parse_key,
            set_key,
            reset_key,
            list_backups,
            make_backup,
            restore_backup
        ])
        .run(tauri::generate_context!())
        .expect("failed to start NuPhy Desktop");
}
