// Sample data so the UI can be developed and previewed in a normal browser.
// Never used inside the desktop app.

const GB = 1e9;
const MB = 1e6;
const HOME = "/Users/demo";
const now = Math.floor(Date.now() / 1000);
const day = 86400;

const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));

const memory = () => {
  const used = 5.8 * GB + Math.random() * 0.6 * GB;
  return {
    total_bytes: 16 * GB,
    used_bytes: used,
    available_bytes: 16 * GB - used,
    swap_total_bytes: 2 * GB,
    swap_used_bytes: 0.4 * GB,
    free_percent: 58,
    pressure: "normal",
    cpu_percent: 14 + Math.random() * 10,
  };
};

const rule = (id: string, name: string, category: string, safety: string, description: string, items: [string, number, number][], extra: Record<string, unknown> = {}) => ({
  rule: { id, name, category, safety, description, command: null, always_permanent: false, ...extra },
  items: items.map(([p, b, age]) => ({ path: `${HOME}/${p}`, name: p.split("/").pop(), bytes: b, modified: now - age * day })),
  total_bytes: items.reduce((a, [, b]) => a + b, 0),
  note: category === "xcode" ? "Xcode isn't installed, so everything here is leftover and safe to remove." : null,
});

const cleanerScan = () => {
  const rules = [
    rule("simulator-devices", "iOS simulators", "xcode", "review", "Every simulator and the apps/data installed in it. Often huge and forgotten, especially after Xcode is removed.", [
      ["Library/Developer/CoreSimulator/Devices/7A1C-iPhone 15 Pro", 14.2 * GB, 200],
      ["Library/Developer/CoreSimulator/Devices/91BD-iPhone 14", 11.8 * GB, 320],
      ["Library/Developer/CoreSimulator/Devices/C0FE-iPad Air", 9.1 * GB, 400],
    ], { command: "xcrun simctl delete unavailable" }),
    rule("simulator-runtimes", "Simulator runtimes (iOS versions)", "xcode", "needs_password", "Downloaded iOS/watchOS runtimes, several GB each. They belong to macOS, so it asks for your password to remove them. Xcode downloads one again if you need it.", [
      ["iOS 17.5 (21F79)", 7.2 * GB, 900],
    ], { always_permanent: true }),
    rule("xcode-derived-data", "Xcode DerivedData", "xcode", "safe", "Build products and indexes. Xcode rebuilds them on the next build.", [
      ["Library/Developer/Xcode/DerivedData/ShopApp-abc", 3.4 * GB, 30],
      ["Library/Developer/Xcode/DerivedData/Wallet-def", 2.1 * GB, 90],
    ]),
    rule("gradle-caches", "Gradle caches", "android", "safe", "Downloaded dependencies and build caches. Gradle re-downloads what a project needs.", [[".gradle/caches", 6.3 * GB, 5]]),
    rule("android-avd", "Android emulators (AVDs)", "android", "review", "Emulator disk images, 2–10 GB each.", [
      [".android/avd/Pixel_7_API_34.avd", 7.8 * GB, 60],
      [".android/avd/Pixel_4_API_30.avd", 4.2 * GB, 400],
    ]),
    rule("npm-cache", "npm cache", "java_script", "safe", "npm's download cache, logs and npx packages.", [[".npm/_cacache", 3.9 * GB, 1]]),
    rule("yarn-cache", "Yarn cache", "java_script", "safe", "Yarn's package cache.", [["Library/Caches/Yarn", 1.6 * GB, 10]]),
    rule("cocoapods", "CocoaPods cache", "languages", "safe", "Downloaded pods.", [["Library/Caches/CocoaPods", 1.1 * GB, 40]]),
    rule("user-caches", "App caches", "system", "safe", "Caches of all your apps. Apps rebuild them.", [
      ["Library/Caches/com.spotify.client", 1.2 * GB, 1],
      ["Library/Caches/Google", 0.8 * GB, 2],
    ]),
    rule("docker", "Docker disk image", "tools", "report_only", "Docker keeps images, containers and volumes in one virtual disk.", [["Library/Containers/com.docker.docker/Data/vms", 18.4 * GB, 1]], { command: "docker system prune -a" }),
  ];
  const total = rules.reduce((a, r) => a + r.total_bytes, 0);
  const safe = rules.filter((r) => r.rule.safety === "safe").reduce((a, r) => a + r.total_bytes, 0);
  return { rules, total_bytes: total, safe_bytes: safe, xcode_installed: false };
};

const summary = () => ({
  root: HOME,
  total_bytes: 118 * GB,
  file_count: 684_211,
  unreadable: 12,
  top_files: [
    ["Downloads/Xcode_15.4.xip", 7.9 * GB, "installer", 300],
    ["Movies/Screen Recording 2024-03-02.mov", 4.1 * GB, "video", 500],
    ["Downloads/ubuntu-24.04-desktop-arm64.iso", 3.2 * GB, "disk_image", 200],
    ["Documents/client-assets-final.zip", 1.9 * GB, "archive", 120],
    ["Downloads/Android Studio.dmg", 1.2 * GB, "disk_image", 250],
    ["Desktop/demo-walkthrough.mp4", 0.9 * GB, "video", 40],
  ].map(([p, b, k, age]) => ({ path: `${HOME}/${p}`, name: String(p).split("/").pop(), bytes: b, kind: k, modified: now - Number(age) * day, accessed: now - Number(age) * day })),
  kinds: [
    { kind: "other", bytes: 61 * GB, count: 600_000 },
    { kind: "video", bytes: 19 * GB, count: 210 },
    { kind: "disk_image", bytes: 14 * GB, count: 9 },
    { kind: "installer", bytes: 9 * GB, count: 12 },
    { kind: "image", bytes: 8 * GB, count: 42_000 },
    { kind: "archive", bytes: 5 * GB, count: 310 },
    { kind: "document", bytes: 2 * GB, count: 1_900 },
  ],
});

const children = (path: string) => {
  const sub: [string, number, boolean][] =
    path === HOME
      ? [["Library", 52 * GB, true], ["Downloads", 24 * GB, true], ["Movies", 11 * GB, true], ["projects", 16 * GB, true], ["Documents", 9 * GB, true], ["Desktop", 4 * GB, true], ["notes.txt", 0.01 * MB, false]]
      : [["build", 2 * GB, true], ["assets", 1.1 * GB, true], ["video.mov", 0.8 * GB, false], ["readme.md", 0.002 * MB, false]];
  return sub.map(([n, b, d]) => ({ path: `${path}/${n}`, name: n, bytes: b, is_dir: d, modified: now - 10 * day }));
};

const proc = (pid: number, name: string, app: string, mem: number, dev: string | null, advice: string | null, command = "") => ({
  pid, name, app, memory_bytes: mem, cpu_percent: Math.random() * 20, command, protected: false, dev_kind: dev, advice, bundle: null,
});

const snapshot = () => ({
  apps: [
    { app: "Google Chrome", memory_bytes: 705 * MB, cpu_percent: 9, process_count: 29, pids: [1], protected: false, bundle: null },
    { app: "Cursor", memory_bytes: 604 * MB, cpu_percent: 4, process_count: 15, pids: [2], protected: false, bundle: null },
    { app: "Claude", memory_bytes: 360 * MB, cpu_percent: 1, process_count: 17, pids: [3], protected: false, bundle: null },
    { app: "Docker", memory_bytes: 182 * MB, cpu_percent: 1, process_count: 8, pids: [4], protected: false, bundle: null },
    { app: "Xcode", memory_bytes: 146 * MB, cpu_percent: 0.4, process_count: 6, pids: [5], protected: false, bundle: null },
    { app: "WindowServer", memory_bytes: 120 * MB, cpu_percent: 6, process_count: 1, pids: [6], protected: true, bundle: null },
  ],
  dev_leftovers: [
    proc(4411, "java", "java", 1.1 * GB, "gradle_daemon", "Idle Gradle daemons keep 0.5–2 GB each. Safe to quit; the next build starts a new one.", "java … org.gradle.launcher.daemon.bootstrap.GradleDaemon 8.7"),
    proc(4520, "qemu-system-aarch64", "qemu-system-aarch64", 2.0 * GB, "android_emulator", "A running Android emulator. Quit it if you're not testing.", "qemu-system-aarch64 -avd Pixel_7_API_34"),
    proc(5102, "node", "node", 0.4 * GB, "dev_server", "A dev server (Metro, Vite, Next, webpack…). Quit it if you forgot it running.", "node node_modules/.bin/react-native start"),
    proc(3301, "adb", "adb", 0.02 * GB, "adb_server", "Android debug bridge. Safe to quit; it restarts when you run adb or Android Studio.", "adb -L tcp:5037 fork-server server"),
  ],
  top_processes: [],
});

const security = () => ({
  findings: [
    { id: "1", severity: "high", area: "startup", title: "Suspicious startup item: com.apple.sysupdate", detail: "This item runs node on a script hidden in /Users/demo/.npl/main.js.", path: `${HOME}/Library/LaunchAgents/com.apple.sysupdate.plist`, line: null, evidence: "/usr/local/bin/node /Users/demo/.npl/main.js", advice: "If you don't recognise it, quarantine it, then run: launchctl bootout gui/$(id -u)/com.apple.sysupdate", can_quarantine: true, fix: null },
    { id: "2", severity: "high", area: "project_code", title: "Suspicious code in a project config file", detail: "This config file hides code far to the right behind a wall of spaces. Fake job-interview projects (Contagious Interview / BeaverTail) hide malware in files like this.", path: `${HOME}/projects/test-task/tailwind.config.js`, line: 14, evidence: "module.exports = {…};                                        eval(atob('Y29uc3Qg…'))", advice: "Don't run npm install / npm start in this project until you've checked the file.", can_quarantine: false, fix: null },
    { id: "3", severity: "medium", area: "git", title: "Suspicious git hook", detail: "This repository hook pushes code to a remote on its own.", path: `${HOME}/projects/shop/.git/hooks/post-commit`, line: 2, evidence: "git push -f origin HEAD >/dev/null 2>&1 &", advice: "Hooks run automatically on commit/push. If you didn't add this, quarantine it.", can_quarantine: true, fix: null },
    { id: "4", severity: "medium", area: "secrets", title: "npm token saved in plain text", detail: "~/.npmrc holds a publish token. npm worms read this file to publish malware under your name.", path: `${HOME}/.npmrc`, line: 1, evidence: "npm_abcd…", advice: "Use a short-lived or read-only token, and enable 2FA for publishing.", can_quarantine: false, fix: null },
    { id: "8", severity: "medium", area: "secrets", title: "Git saves passwords in plain text", detail: "credential.helper=store keeps your GitHub token unencrypted in ~/.git-credentials.", path: `${HOME}/.git-credentials`, line: null, evidence: null, advice: "Use the Keychain instead: git config --global credential.helper osxkeychain", can_quarantine: false, fix: "use_keychain_credentials" },
    { id: "5", severity: "low", area: "git", title: "Repository commits under a different email", detail: "Commits in this repo are signed as <someone@else.com>, not your usual <you@company.com>.", path: `${HOME}/projects/api`, line: null, evidence: null, advice: null, can_quarantine: false, fix: null },
    { id: "6", severity: "info", area: "git", title: "Your git identity", detail: "New commits are signed as Demo User <you@company.com>. If this isn't you, something changed your git config.", path: null, line: null, evidence: null, advice: null, can_quarantine: false, fix: null },
    { id: "7", severity: "info", area: "startup", title: "Startup item: com.google.keystone.agent", detail: "This item starts automatically.", path: `${HOME}/Library/LaunchAgents/com.google.keystone.agent.plist`, line: null, evidence: "/Users/demo/Library/Google/GoogleSoftwareUpdate/…/GoogleSoftwareUpdateAgent", advice: null, can_quarantine: false, fix: null },
  ],
  scanned_repos: 23,
  scanned_projects: 41,
  scanned_packages: 18_230,
  duration_ms: 4210,
  counts: { high: 2, medium: 3, low: 1, info: 2 },
});

const deleted = (paths: string[]) => ({ removed: paths.map((p) => ({ path: p, bytes: 1.5 * GB })), failed: [], bytes_freed: paths.length * 1.5 * GB });

export async function mock<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const a = args ?? {};
  const r = (v: unknown, ms = 150) => wait(ms).then(() => v as T);
  switch (cmd) {
    case "system_info":
      return r({ hostname: "demo-mbp", os_name: "macOS", os_version: "macOS 15.3 Sequoia", model: "MacBookAir10,1", cpu_brand: "Apple M1", cpu_cores: 8, uptime_secs: 3 * day + 5000, disk: { name: "Macintosh HD", mount: "/System/Volumes/Data", total_bytes: 1000 * GB, free_bytes: 658 * GB }, memory: memory() });
    case "memory_live": return r(memory(), 20);
    case "device_info": return r({ name: "MacBook Pro", chip: "Apple M2 Pro", memory_bytes: 16 * GB, os_label: "macOS Sonoma 14.5", kind: "laptop" }, 80);
    case "memory_breakdown": return r({ total: 16 * GB, apps: 4.2 * GB, wired: 1.1 * GB, compressed: 0.5 * GB, free: 10.2 * GB }, 20);
    case "app_icon": return r(null, 0);
    case "folder_explain": {
      const name = String(a.path).split("/").pop() ?? "";
      const known: Record<string, [string, string, string]> = {
        Library: ["Your Library", "Settings, caches and data for your apps and macOS. Never delete the folder itself; the Clean page clears the safe parts inside it.", "leave_it"],
        Downloads: ["Downloads", "Files you downloaded. Often full of old installers (.dmg, .pkg, .zip) you no longer need.", "yours"],
        Movies: ["Movies", "Your videos and screen recordings, often large.", "yours"],
        Documents: ["Documents", "Your documents. Check before deleting anything here.", "yours"],
        Desktop: ["Desktop", "Files on your desktop.", "yours"],
      };
      const k = known[name];
      return r(k ? { title: k[0], text: k[1], advice: k[2] } : null, 50);
    }
    case "home_dir": return r(HOME, 0);
    case "has_full_disk_access": return r(false, 0);
    case "cleaner_scan": return r(cleanerScan(), 1200);
    case "cleaner_clean": {
      const reqs = a.requests as { rule_id: string; paths: string[] | null }[];
      return r(deleted(reqs.flatMap((q) => q.paths ?? [q.rule_id])), 800);
    }
    case "node_modules_find":
      return r([
        { path: `${HOME}/projects/old-landing/node_modules`, project: `${HOME}/projects/old-landing`, bytes: 812 * MB, last_touched: now - 220 * day, stale: true, in_cloud: false },
        { path: `${HOME}/projects/rn-demo/node_modules`, project: `${HOME}/projects/rn-demo`, bytes: 1.4 * GB, last_touched: now - 95 * day, stale: true, in_cloud: false },
        { path: `${HOME}/Documents/Cursor project/landing/node_modules`, project: `${HOME}/Documents/Cursor project/landing`, bytes: 0, last_touched: now - 16 * day, stale: false, in_cloud: true },
        { path: `${HOME}/projects/shop/node_modules`, project: `${HOME}/projects/shop`, bytes: 690 * MB, last_touched: now - 2 * day, stale: false, in_cloud: false },
      ], 900);
    case "node_modules_remove":
    case "bigfiles_remove": return r(deleted(a.paths as string[]), 500);
    case "bigfiles_scan": return r(summary(), 1500);
    case "bigfiles_summary": return r(null, 0);
    case "bigfiles_children": return r(children(a.path as string), 60);
    case "memory_snapshot": return r(snapshot(), 100);
    case "process_quit": return r({ pid: a.pid, name: a.name, ok: true, message: "Asked to quit." });
    case "app_quit": return r([{ pid: 1, name: a.app, ok: true, message: "Asked the app to quit (it may ask to save)." }]);
    case "security_scan": return r(security(), 1500);
    case "security_quarantine": return r({ id: "q1", original: "x", stored: "y", reason: "z", at: now, mode: null, stopped: true });
    case "security_fix": return r("Git now uses the Keychain. Delete ~/.git-credentials after your next successful push.", 300);
    case "open_settings": return r(undefined, 0);
    case "protection_checks": return r([
      { id: "filevault", title: "FileVault disk encryption", about: "Keeps your files unreadable if your Mac is lost or stolen.", state: "pass", pane: "file_vault", how: null },
      { id: "firewall", title: "Firewall", about: "Blocks unwanted incoming connections.", state: "fail", pane: "firewall", how: null },
      { id: "gatekeeper", title: "Gatekeeper", about: "Only lets apps from identified developers open.", state: "pass", pane: "privacy_security", how: null },
      { id: "sip", title: "System Integrity Protection", about: "Stops anything, even with your password, from changing macOS system files.", state: "pass", pane: null, how: "Restart into Recovery, open Terminal, run `csrutil enable`, then restart." },
      { id: "updates", title: "Automatic macOS updates", about: "Installs security fixes as soon as Apple ships them.", state: "unknown", pane: "software_update", how: "Click the ⓘ next to Automatic updates and turn on \"Install macOS updates\"." },
      { id: "remote_login", title: "Remote Login (SSH) off", about: "When on, anyone with your password can log in over the network.", state: "pass", pane: "sharing", how: null },
    ], 400);
    case "quarantine_list": return r([], 50);
    default: return r(undefined, 50);
  }
}
