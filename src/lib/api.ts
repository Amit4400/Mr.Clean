import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import { mock } from "./mock";
import type {
  CleanerScan,
  DeviceInfo,
  Explanation,
  MemoryBreakdown,
  CleanRequest,
  DeleteMode,
  DeleteReport,
  MemoryInfo,
  MemorySnapshot,
  Node,
  NodeModulesHit,
  QuarantineEntry,
  QuitResult,
  ScanProgress,
  SecurityReport,
  Summary,
  SystemInfo,
} from "./types";

/** True inside the desktop app; false in a plain browser (uses mock data). */
export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!inTauri) return mock<T>(cmd, args);
  return invoke<T>(cmd, args);
}

export const api = {
  systemInfo: () => call<SystemInfo>("system_info"),
  memoryLive: () => call<MemoryInfo>("memory_live"),
  deviceInfo: () => call<DeviceInfo>("device_info"),
  memoryBreakdown: () => call<MemoryBreakdown>("memory_breakdown"),
  /** PNG data URL of an app's icon: by `.app` path, or by installed app name. */
  appIcon: (q: { bundle?: string | null; name?: string }) =>
    call<string | null>("app_icon", { bundle: q.bundle ?? null, name: q.name ?? null }),
  homeDir: () => call<string>("home_dir"),
  /** Plain-English "what is this folder?" (null when unknown). */
  folderExplain: (path: string) => call<Explanation | null>("folder_explain", { path }),
  hasFullDiskAccess: () => call<boolean>("has_full_disk_access"),
  cancelScan: () => call<void>("cancel_scan"),
  reveal: (path: string) => call<void>("reveal", { path }),

  cleanerScan: () => call<CleanerScan>("cleaner_scan"),
  cleanerClean: (requests: CleanRequest[], mode: DeleteMode) =>
    call<DeleteReport>("cleaner_clean", { requests, mode }),
  nodeModulesFind: (staleDays: number) => call<NodeModulesHit[]>("node_modules_find", { staleDays }),
  nodeModulesRemove: (paths: string[], mode: DeleteMode) =>
    call<DeleteReport>("node_modules_remove", { paths, mode }),

  bigfilesScan: (root?: string) => call<Summary>("bigfiles_scan", { root: root ?? null }),
  bigfilesChildren: (path: string) => call<Node[]>("bigfiles_children", { path }),
  bigfilesRemove: (paths: string[], mode: DeleteMode) =>
    call<DeleteReport>("bigfiles_remove", { paths, mode }),
  bigfilesSummary: () => call<Summary | null>("bigfiles_summary"),

  memorySnapshot: () => call<MemorySnapshot>("memory_snapshot"),
  processQuit: (pid: number, name: string, force: boolean) =>
    call<QuitResult>("process_quit", { pid, name, force }),
  appQuit: (app: string, force: boolean) => call<QuitResult[]>("app_quit", { app, force }),

  securityScan: () => call<SecurityReport>("security_scan"),
  securityQuarantine: (findingId: string) => call<QuarantineEntry>("security_quarantine", { findingId }),
  quarantineList: () => call<QuarantineEntry[]>("quarantine_list"),
  quarantineRestore: (id: string) => call<void>("quarantine_restore", { id }),
};

export async function onScanProgress(cb: (p: ScanProgress) => void): Promise<UnlistenFn> {
  if (!inTauri) return () => {};
  return listen<ScanProgress>("scan-progress", (e) => cb(e.payload));
}

export function openFullDiskAccessSettings() {
  return call<void>("open_full_disk_access_settings");
}
