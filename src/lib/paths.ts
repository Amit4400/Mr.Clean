/** Mirrors the Rust user-file guard so we don't offer checkboxes it would refuse. */
export function isProtected(path: string, isDir: boolean, home: string | null): boolean {
  if (!home) return false;
  // Windows paths use backslashes and ignore case.
  const windows = /^[A-Za-z]:[\\/]/.test(home) || home.includes("\\");
  const norm = (p: string) => {
    const s = p.replace(/\\/g, "/").replace(/\/+$/, "");
    return windows ? s.toLowerCase() : s;
  };
  const h = norm(home);
  const p = norm(path);
  const rel = p.startsWith(h + "/") ? p.slice(h.length + 1) : null;
  if (rel === null) return true;
  const under = (d: string) => rel === norm(d) || rel.startsWith(norm(d) + "/");
  if (isDir && !rel.includes("/")) return true;
  if (!rel.includes("/") && rel.toLowerCase().startsWith("ntuser")) return true;
  if (rel.split("/").includes(".git")) return true;
  if ([".ssh", ".gnupg", ".aws", ".kube", ".mrclean"].some(under)) return true;
  if (under("Library")) return !["Library/Caches", "Library/Logs", "Library/Developer"].some(under);
  if (under("AppData")) return !["AppData/Local/Temp", "AppData/Local/CrashDumps"].some(under);
  return false;
}
