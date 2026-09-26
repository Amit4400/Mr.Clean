//! Tauri commands: thin wrappers that run core functions off the UI thread
//! and stream progress events while long scans run.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use mrclean_core::bigfiles::{self, BigScan, Node, Summary};
use mrclean_core::cleaner::{self, node_modules, CleanRequest, CleanerScan};
use mrclean_core::fsutil::Progress;
use mrclean_core::memory::{self, MemorySnapshot, QuitResult};
use mrclean_core::safety::{DeleteMode, DeleteReport};
use mrclean_core::security::protection::{self, Pane};
use mrclean_core::security::{self, QuarantineEntry, SecurityReport};
use mrclean_core::sysinfo::System;
use mrclean_core::system::{self, DeviceInfo, MemoryInfo, SystemInfo};
use mrclean_core::{Cancel, Env};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

#[derive(Default)]
struct AppState {
    sys: Mutex<System>,
    cancel: Cancel,
    bigscan: Mutex<Option<BigScan>>,
    security: Mutex<Option<SecurityReport>>,
}

type Res<T> = Result<T, String>;

#[derive(Clone, Serialize)]
struct ProgressEvent {
    task: &'static str,
    files: u64,
    bytes: u64,
}

/// Run `work` on a blocking thread, emitting `scan-progress` every 250 ms.
async fn with_progress<T, F>(app: AppHandle, task: &'static str, work: F) -> Res<T>
where
    T: Send + 'static,
    F: FnOnce(&Progress) -> T + Send + 'static,
{
    let progress = Arc::new(Progress::default());
    let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
    {
        let (progress, done, app) = (progress.clone(), done.clone(), app.clone());
        std::thread::spawn(move || {
            while !done.load(std::sync::atomic::Ordering::Relaxed) {
                let (files, bytes) = progress.snapshot();
                let _ = app.emit("scan-progress", ProgressEvent { task, files, bytes });
                std::thread::sleep(Duration::from_millis(250));
            }
        });
    }
    let result = tauri::async_runtime::spawn_blocking(move || {
        let r = work(&progress);
        done.store(true, std::sync::atomic::Ordering::Relaxed);
        r
    })
    .await
    .map_err(|e| e.to_string())?;
    Ok(result)
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Res<T> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())
}

// ------------------------------------------------------------- dashboard

#[tauri::command]
async fn system_info(state: State<'_, AppState>) -> Res<SystemInfo> {
    let mut sys = state.sys.lock().map_err(|e| e.to_string())?;
    sys.refresh_cpu_all();
    sys.refresh_memory();
    Ok(system::info(&sys))
}

#[tauri::command]
fn memory_live(state: State<'_, AppState>) -> Res<MemoryInfo> {
    let mut sys = state.sys.lock().map_err(|e| e.to_string())?;
    sys.refresh_memory();
    sys.refresh_cpu_usage();
    Ok(system::memory(&sys))
}

#[tauri::command]
async fn device_info() -> Res<DeviceInfo> {
    blocking(|| {
        let mut sys = System::new();
        sys.refresh_memory();
        sys.refresh_cpu_all();
        system::device(&sys)
    })
    .await
}

#[tauri::command]
fn memory_breakdown(state: State<'_, AppState>) -> Res<memory::Breakdown> {
    let mut sys = state.sys.lock().map_err(|e| e.to_string())?;
    sys.refresh_memory();
    Ok(memory::breakdown(&sys))
}

/// Icon of an app bundle (a path ending in `.app`), or of an installed app by name.
#[tauri::command]
async fn app_icon(bundle: Option<String>, name: Option<String>) -> Res<Option<String>> {
    blocking(move || {
        let env = Env::detect();
        let path = match (bundle, name) {
            (Some(b), _) => Some(PathBuf::from(b)),
            (None, Some(n)) => mrclean_core::icons::find_app(&env, &n),
            _ => None,
        }?;
        mrclean_core::icons::icon_data_url(&path, &mrclean_core::icons::default_cache_dir())
    })
    .await
}

/// "What is this folder?" for the Files page. Read-only.
#[tauri::command]
fn folder_explain(path: String) -> Option<mrclean_core::explain::Explanation> {
    mrclean_core::explain::explain(&Env::detect(), std::path::Path::new(&path))
}

#[tauri::command]
fn home_dir() -> String {
    Env::detect().home.display().to_string()
}

/// macOS hides parts of ~/Library until the app has Full Disk Access.
#[tauri::command]
fn has_full_disk_access() -> bool {
    if !cfg!(target_os = "macos") {
        return true;
    }
    let home = Env::detect().home;
    [
        "Library/Safari",
        "Library/Mail",
        "Library/Application Support/com.apple.TCC",
    ]
    .iter()
    .map(|p| home.join(p))
    .filter(|p| p.exists())
    .all(|p| std::fs::read_dir(p).is_ok())
}

#[tauri::command]
fn open_full_disk_access_settings() -> Res<()> {
    open_settings(Pane::FullDiskAccess)
}

/// Open one of the allowlisted System Settings pages (never an arbitrary URL).
#[tauri::command]
fn open_settings(pane: Pane) -> Res<()> {
    tauri_plugin_opener::open_url(pane.url(), None::<&str>).map_err(|e| e.to_string())
}

/// FileVault, Firewall, Gatekeeper, SIP, updates and Remote Login. Read-only.
#[tauri::command]
async fn protection_checks() -> Res<Vec<protection::Check>> {
    blocking(protection::checks).await
}

/// Apply the one-click fix of a finding from the last scan (user-confirmed).
#[tauri::command]
fn security_fix(state: State<'_, AppState>, finding_id: String) -> Res<String> {
    let guard = state.security.lock().map_err(|e| e.to_string())?;
    let report = guard.as_ref().ok_or("Run a scan first.")?;
    let fix = report
        .findings
        .iter()
        .find(|f| f.id == finding_id)
        .and_then(|f| f.fix)
        .ok_or("This finding has no automatic fix.")?;
    security::apply_fix(&Env::detect(), fix)
}

#[tauri::command]
fn cancel_scan(state: State<'_, AppState>) {
    state.cancel.cancel();
}

#[tauri::command]
fn reveal(path: String) -> Res<()> {
    tauri_plugin_opener::reveal_item_in_dir(PathBuf::from(path)).map_err(|e| e.to_string())
}

// --------------------------------------------------------------- cleaner

#[tauri::command]
async fn cleaner_scan(app: AppHandle, state: State<'_, AppState>) -> Res<CleanerScan> {
    let cancel = state.cancel.clone();
    cancel.reset();
    with_progress(app, "cleaner", move |p| cleaner::scan(&Env::detect(), &cancel, p)).await
}

#[tauri::command]
async fn cleaner_clean(requests: Vec<CleanRequest>, mode: DeleteMode) -> Res<DeleteReport> {
    blocking(move || cleaner::clean(&Env::detect(), &requests, mode)).await
}

#[tauri::command]
async fn node_modules_find(state: State<'_, AppState>, stale_days: u32) -> Res<Vec<node_modules::NodeModulesHit>> {
    let cancel = state.cancel.clone();
    cancel.reset();
    blocking(move || {
        let env = Env::detect();
        let home = env.home.clone();
        node_modules::find(&env, &[home], stale_days, &cancel)
    })
    .await
}

#[tauri::command]
async fn node_modules_remove(paths: Vec<String>, mode: DeleteMode) -> Res<DeleteReport> {
    blocking(move || node_modules::remove(&Env::detect(), &paths, mode)).await
}

// ------------------------------------------------------------ large files

#[tauri::command]
async fn bigfiles_scan(app: AppHandle, state: State<'_, AppState>, root: Option<String>) -> Res<Summary> {
    let cancel = state.cancel.clone();
    cancel.reset();
    let root = root.map(PathBuf::from).unwrap_or_else(|| Env::detect().home);
    if !root.is_dir() {
        return Err("That folder doesn't exist.".into());
    }
    let scan = with_progress(app, "bigfiles", move |p| bigfiles::scan(&root, &cancel, p)).await?;
    let summary = scan.summary();
    *state.bigscan.lock().map_err(|e| e.to_string())? = Some(scan);
    Ok(summary)
}

#[tauri::command]
fn bigfiles_children(state: State<'_, AppState>, path: String) -> Res<Vec<Node>> {
    let guard = state.bigscan.lock().map_err(|e| e.to_string())?;
    let scan = guard.as_ref().ok_or("Run a scan first.")?;
    Ok(scan.children(&PathBuf::from(path)))
}

#[tauri::command]
async fn bigfiles_remove(state: State<'_, AppState>, paths: Vec<String>, mode: DeleteMode) -> Res<DeleteReport> {
    let report = blocking(move || bigfiles::remove(&Env::detect(), &paths, mode)).await?;
    if let Some(scan) = state.bigscan.lock().map_err(|e| e.to_string())?.as_mut() {
        for r in &report.removed {
            scan.forget(&PathBuf::from(&r.path), r.bytes);
        }
    }
    Ok(report)
}

#[tauri::command]
fn bigfiles_summary(state: State<'_, AppState>) -> Res<Option<Summary>> {
    Ok(state
        .bigscan
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .map(BigScan::summary))
}

// ----------------------------------------------------------------- memory

#[tauri::command]
fn memory_snapshot(state: State<'_, AppState>) -> Res<MemorySnapshot> {
    let mut sys = state.sys.lock().map_err(|e| e.to_string())?;
    memory::refresh(&mut sys);
    Ok(memory::snapshot(&sys))
}

#[tauri::command]
fn process_quit(state: State<'_, AppState>, pid: u32, name: String, force: bool) -> Res<QuitResult> {
    let mut sys = state.sys.lock().map_err(|e| e.to_string())?;
    memory::refresh(&mut sys);
    Ok(memory::quit(&sys, pid, &name, force))
}

#[tauri::command]
fn app_quit(state: State<'_, AppState>, app: String, force: bool) -> Res<Vec<QuitResult>> {
    let mut sys = state.sys.lock().map_err(|e| e.to_string())?;
    memory::refresh(&mut sys);
    Ok(memory::quit_app(&sys, &app, force))
}

// --------------------------------------------------------------- security

#[tauri::command]
async fn security_scan(state: State<'_, AppState>) -> Res<SecurityReport> {
    let cancel = state.cancel.clone();
    cancel.reset();
    let report = blocking(move || {
        let env = Env::detect();
        let roots = vec![env.home.clone()];
        security::scan(&env, &roots, &cancel)
    })
    .await?;
    *state.security.lock().map_err(|e| e.to_string())? = Some(report.clone());
    Ok(report)
}

/// Quarantine by finding id: only paths the last scan flagged as quarantinable.
#[tauri::command]
fn security_quarantine(state: State<'_, AppState>, finding_id: String) -> Res<QuarantineEntry> {
    let guard = state.security.lock().map_err(|e| e.to_string())?;
    let report = guard.as_ref().ok_or("Run a scan first.")?;
    let f = report
        .findings
        .iter()
        .find(|f| f.id == finding_id && f.can_quarantine)
        .ok_or("This finding can't be quarantined.")?;
    let path = f.path.as_ref().ok_or("This finding has no file.")?;
    security::quarantine(&Env::detect(), &PathBuf::from(path), &f.title)
}

#[tauri::command]
fn quarantine_list() -> Vec<QuarantineEntry> {
    security::quarantined(&Env::detect())
}

#[tauri::command]
fn quarantine_restore(id: String) -> Res<()> {
    security::restore(&Env::detect(), &id)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            system_info,
            memory_live,
            device_info,
            memory_breakdown,
            app_icon,
            home_dir,
            folder_explain,
            has_full_disk_access,
            open_full_disk_access_settings,
            open_settings,
            protection_checks,
            security_fix,
            cancel_scan,
            reveal,
            cleaner_scan,
            cleaner_clean,
            node_modules_find,
            node_modules_remove,
            bigfiles_scan,
            bigfiles_children,
            bigfiles_remove,
            bigfiles_summary,
            memory_snapshot,
            process_quit,
            app_quit,
            security_scan,
            security_quarantine,
            quarantine_list,
            quarantine_restore,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Mr.Clean");
}
