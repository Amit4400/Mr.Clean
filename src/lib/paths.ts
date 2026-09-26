/** Mirrors the Rust user-file guard so we don't offer checkboxes it would refuse. */
export function isProtected(path: string, isDir: boolean, home: string | null): boolean {
  if (!home) return false;
  const rel = path.startsWith(home + "/") ? path.slice(home.length + 1) : null;
  if (rel === null) return true;
  if (isDir && !rel.includes("/")) return true;
  if (rel.split("/").includes(".git")) return true;
  if ([".ssh", ".gnupg", ".aws", ".kube", ".mrclean"].some((d) => rel === d || rel.startsWith(d + "/"))) return true;
  if (rel.startsWith("Library/")) return !["Library/Caches/", "Library/Logs/", "Library/Developer/"].some((d) => rel.startsWith(d));
  return false;
}
