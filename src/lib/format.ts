export function bytes(n: number, digits = 1): string {
  if (!Number.isFinite(n) || n <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.min(units.length - 1, Math.floor(Math.log(n) / Math.log(1000)));
  const v = n / 1000 ** i;
  return `${v.toFixed(i === 0 ? 0 : v >= 100 ? 0 : digits)} ${units[i]}`;
}

export function ago(unixSecs: number | null | undefined): string {
  if (!unixSecs) return "—";
  const s = Date.now() / 1000 - unixSecs;
  if (s < 60) return "just now";
  const mins = s / 60;
  if (mins < 60) return `${Math.round(mins)} min ago`;
  const hours = mins / 60;
  if (hours < 24) return `${Math.round(hours)} h ago`;
  const days = hours / 24;
  if (days < 45) return `${Math.round(days)} days ago`;
  const months = days / 30.4;
  if (months < 18) return `${Math.round(months)} months ago`;
  return `${(days / 365).toFixed(1)} years ago`;
}

export function duration(secs: number): string {
  const d = Math.floor(secs / 86400);
  const h = Math.floor((secs % 86400) / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

/** "/Users/me/Library/Caches/x" → "~/Library/Caches/x" */
export function tildify(path: string, home: string | null): string {
  if (home && path.startsWith(home)) return "~" + path.slice(home.length);
  return path;
}

export function percent(part: number, whole: number): number {
  return whole > 0 ? Math.round((part / whole) * 100) : 0;
}
