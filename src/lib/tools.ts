import type { Category } from "./types";

/** Filter chips on the Clean page, in display order. */
export const CHIPS = ["xcode", "android", "javascript", "python", "rust", "homebrew", "ide", "logs", "other"] as const;
export type ChipId = (typeof CHIPS)[number];

export const CHIP_LABEL: Record<ChipId, string> = {
  xcode: "Xcode",
  android: "Android",
  javascript: "npm & JS",
  python: "Python",
  rust: "Rust",
  homebrew: "Homebrew",
  ide: "IDE",
  logs: "Logs",
  other: "Other",
};

const CHIP_BY_RULE: Record<string, ChipId> = {
  pip: "python",
  cargo: "rust",
  homebrew: "homebrew",
  vscode: "ide",
  jetbrains: "ide",
  "user-logs": "logs",
  "gradle-daemon": "logs",
  journal: "logs",
  "win-crash-dumps": "logs",
};

/** Which chip a cleaner rule belongs to. */
export function chipOf(ruleId: string, category: Category): ChipId {
  if (CHIP_BY_RULE[ruleId]) return CHIP_BY_RULE[ruleId];
  if (category === "xcode") return "xcode";
  if (category === "android") return "android";
  if (category === "java_script") return "javascript";
  return "other";
}

// Brand logo (bundled Simple Icons slug) for each rule; the rest get a generic icon.
const TOOL_BY_RULE: Record<string, string> = {
  "simulator-devices": "xcode",
  "simulator-caches": "xcode",
  "simulator-runtimes": "xcode",
  "gradle-caches": "gradle",
  "gradle-wrapper": "gradle",
  "gradle-daemon": "gradle",
  "kotlin-daemon": "kotlin",
  "npm-cache": "npm",
  "npm-cache-win": "npm",
  "yarn-cache": "yarn",
  "pnpm-store": "pnpm",
  "bun-cache": "bun",
  "node-gyp": "nodedotjs",
  "browser-test-tools": "googlechrome",
  "electron-cache": "electron",
  cocoapods: "cocoapods",
  swiftpm: "swift",
  "flutter-pub": "flutter",
  pip: "python",
  cargo: "rust",
  "go-build": "go",
  "go-mod": "go",
  homebrew: "homebrew",
  vscode: "cursor",
  jetbrains: "jetbrains",
  docker: "docker",
  "ios-backups": "apple",
  nuget: "nuget",
  "apt-cache": "debian",
  journal: "linux",
  flatpak: "flatpak",
  snap: "snapcraft",
};

/** Simple Icons slug for a rule, or null for a generic icon. */
export function toolOf(ruleId: string): string | null {
  if (TOOL_BY_RULE[ruleId]) return TOOL_BY_RULE[ruleId];
  if (ruleId.startsWith("xcode-")) return "xcode";
  if (ruleId.startsWith("android-")) return "android";
  return null;
}
