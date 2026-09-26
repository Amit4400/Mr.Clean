import type { DeviceKind } from "../lib/types";

// Simple vector Mac illustrations with a soft macOS-style wallpaper.
function Wallpaper({ id }: { id: string }) {
  return (
    <defs>
      <linearGradient id={`${id}-wp`} x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" stopColor="#1e3a8a" />
        <stop offset="0.45" stopColor="#3b82f6" />
        <stop offset="1" stopColor="#a78bfa" />
      </linearGradient>
      <linearGradient id={`${id}-alu`} x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stopColor="#e5e7eb" />
        <stop offset="1" stopColor="#9ca3af" />
      </linearGradient>
    </defs>
  );
}

function Screen({ id, x, y, w, h }: { id: string; x: number; y: number; w: number; h: number }) {
  return (
    <>
      <rect x={x} y={y} width={w} height={h} rx={2} fill={`url(#${id}-wp)`} />
      <path d={`M${x} ${y + h * 0.72} C ${x + w * 0.3} ${y + h * 0.45}, ${x + w * 0.6} ${y + h * 0.95}, ${x + w} ${y + h * 0.6} L ${x + w} ${y + h} L ${x} ${y + h} Z`} fill="#60a5fa" opacity="0.55" />
      <path d={`M${x} ${y + h * 0.85} C ${x + w * 0.35} ${y + h * 0.7}, ${x + w * 0.7} ${y + h}, ${x + w} ${y + h * 0.8} L ${x + w} ${y + h} L ${x} ${y + h} Z`} fill="#c4b5fd" opacity="0.5" />
    </>
  );
}

export function DeviceArt({ kind, width = 104 }: { kind: DeviceKind; width?: number }) {
  const id = `dev-${kind}`;
  if (kind === "laptop") {
    return (
      <svg viewBox="0 0 120 76" width={width} aria-hidden>
        <Wallpaper id={id} />
        <rect x="16" y="4" width="88" height="58" rx="5" fill="#1f2937" />
        <Screen id={id} x={19} y={7} w={82} h={52} />
        <path d="M4 63 H116 L111 70 Q110 72 107 72 H13 Q10 72 9 70 Z" fill={`url(#${id}-alu)`} />
        <rect x="50" y="63" width="20" height="2.5" rx="1.25" fill="#6b7280" />
      </svg>
    );
  }
  if (kind === "imac" || kind === "desktop") {
    return (
      <svg viewBox="0 0 120 92" width={width * 0.85} aria-hidden>
        <Wallpaper id={id} />
        <rect x="6" y="2" width="108" height="68" rx="5" fill={kind === "imac" ? "#d1d5db" : "#1f2937"} />
        <Screen id={id} x={10} y={6} w={100} h={56} />
        <path d="M48 70 H72 L76 86 H44 Z" fill={`url(#${id}-alu)`} />
        <rect x="36" y="86" width="48" height="4" rx="2" fill="#9ca3af" />
      </svg>
    );
  }
  // Mac mini / Studio: a small aluminium box.
  const tall = kind === "studio";
  return (
    <svg viewBox="0 0 120 70" width={width} aria-hidden>
      <Wallpaper id={id} />
      <rect x="18" y={tall ? 14 : 30} width="84" height={tall ? 44 : 26} rx="8" fill={`url(#${id}-alu)`} />
      <rect x="18" y={tall ? 54 : 52} width="84" height="4" rx="2" fill="#6b7280" opacity="0.5" />
      <circle cx="92" cy={tall ? 46 : 46} r="1.6" fill="#22c55e" />
    </svg>
  );
}
