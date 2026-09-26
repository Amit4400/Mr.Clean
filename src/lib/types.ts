// Mirrors the Rust types in crates/core (serde output).

export type Pressure = "normal" | "warning" | "critical";
export type DeleteMode = "trash" | "permanent";

export interface MemoryInfo {
  total_bytes: number;
  used_bytes: number;
  available_bytes: number;
  swap_total_bytes: number;
  swap_used_bytes: number;
  free_percent: number | null;
  pressure: Pressure;
  cpu_percent: number;
}

export type DeviceKind = "laptop" | "imac" | "mini" | "studio" | "desktop";

export interface DeviceInfo {
  name: string;
  chip: string;
  memory_bytes: number;
  os_label: string;
  kind: DeviceKind;
}

export interface MemoryBreakdown {
  total: number;
  apps: number;
  wired: number;
  compressed: number;
  free: number;
}

export interface DiskInfo {
  name: string;
  mount: string;
  total_bytes: number;
  free_bytes: number;
}

export interface SystemInfo {
  hostname: string;
  os_name: string;
  os_version: string;
  model: string | null;
  cpu_brand: string;
  cpu_cores: number;
  uptime_secs: number;
  disk: DiskInfo | null;
  memory: MemoryInfo;
}

export type Category = "xcode" | "android" | "java_script" | "languages" | "tools" | "system";
export type Safety = "safe" | "review" | "report_only" | "needs_password";

export interface RuleInfo {
  id: string;
  name: string;
  category: Category;
  description: string;
  safety: Safety;
  command: string | null;
  always_permanent: boolean;
}

export interface Item {
  path: string;
  name: string;
  bytes: number;
  modified: number | null;
}

export interface RuleScan {
  rule: RuleInfo;
  items: Item[];
  total_bytes: number;
  note: string | null;
}

export interface CleanerScan {
  rules: RuleScan[];
  total_bytes: number;
  safe_bytes: number;
  xcode_installed: boolean | null;
}

export interface CleanRequest {
  rule_id: string;
  paths: string[] | null;
}

export interface DeleteReport {
  removed: { path: string; bytes: number }[];
  failed: { path: string; error: string }[];
  bytes_freed: number;
}

export interface NodeModulesHit {
  path: string;
  project: string;
  bytes: number;
  last_touched: number | null;
  stale: boolean;
  in_cloud: boolean;
}

export type Kind = "video" | "image" | "audio" | "archive" | "disk_image" | "installer" | "document" | "other";

export interface FileEntry {
  path: string;
  name: string;
  bytes: number;
  kind: Kind;
  modified: number | null;
  accessed: number | null;
}

export interface Node {
  path: string;
  name: string;
  bytes: number;
  is_dir: boolean;
  modified: number | null;
}

export interface Summary {
  root: string;
  total_bytes: number;
  file_count: number;
  unreadable: number;
  top_files: FileEntry[];
  kinds: { kind: Kind; bytes: number; count: number }[];
}

export type DevKind =
  | "gradle_daemon"
  | "kotlin_daemon"
  | "adb_server"
  | "android_emulator"
  | "ios_simulator"
  | "dev_server"
  | "orphan_node"
  | "docker"
  | "language_server";

export interface ProcInfo {
  pid: number;
  name: string;
  app: string;
  memory_bytes: number;
  cpu_percent: number;
  command: string;
  protected: boolean;
  dev_kind: DevKind | null;
  advice: string | null;
  bundle: string | null;
}

export interface AppGroup {
  app: string;
  memory_bytes: number;
  cpu_percent: number;
  process_count: number;
  pids: number[];
  protected: boolean;
  bundle: string | null;
}

export interface MemorySnapshot {
  apps: AppGroup[];
  dev_leftovers: ProcInfo[];
  top_processes: ProcInfo[];
}

export interface QuitResult {
  pid: number;
  name: string;
  ok: boolean;
  message: string;
}

export type Severity = "info" | "low" | "medium" | "high";
export type Area = "known_malware" | "npm" | "git" | "startup" | "shell_profile" | "secrets" | "project_code";

export interface Finding {
  id: string;
  severity: Severity;
  area: Area;
  title: string;
  detail: string;
  path: string | null;
  line: number | null;
  evidence: string | null;
  advice: string | null;
  can_quarantine: boolean;
  fix: Fix | null;
}

export type Fix = "unset_global_hooks_path" | "use_keychain_credentials";

export type Pane = "full_disk_access" | "file_vault" | "firewall" | "privacy_security" | "software_update" | "sharing";

export interface ProtectionCheck {
  id: string;
  title: string;
  about: string;
  state: "pass" | "fail" | "unknown";
  pane: Pane | null;
  how: string | null;
}

export interface SecurityReport {
  findings: Finding[];
  scanned_repos: number;
  scanned_projects: number;
  scanned_packages: number;
  duration_ms: number;
  counts: { high: number; medium: number; low: number; info: number };
}

export interface QuarantineEntry {
  id: string;
  original: string;
  stored: string;
  reason: string;
  at: number;
  mode: number | null;
  stopped: boolean;
}

export interface ScanProgress {
  task: string;
  files: number;
  bytes: number;
}

export type Advice = "safe_to_delete" | "use_the_app" | "leave_it" | "yours" | "check_first";

export interface Explanation {
  title: string;
  text: string;
  advice: Advice;
}
