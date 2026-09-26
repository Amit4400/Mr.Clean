import { HardDrive } from "lucide-react";
import { motion } from "motion/react";

import { bytes } from "../lib/format";
import { AnimatedNumber } from "./ui";

/**
 * The big glassy storage ring: a frosted disc with slowly drifting light
 * inside, and a green ring filling to the used share of the disk.
 * Decoration pauses with the window and stops under Reduce motion.
 */
export function StorageBlob({ used, total, size = 300 }: { used: number | null; total: number | null; size?: number }) {
  const pct = used !== null && total ? Math.min(100, (used / total) * 100) : 0;
  const free = used !== null && total ? total - used : null;
  const r = 118;
  const c = 2 * Math.PI * r;

  return (
    <div className="relative" style={{ width: size, height: size }}>
      <svg viewBox="0 0 300 300" width={size} height={size} role="img" aria-label={`Disk ${Math.round(pct)}% used`}>
        <defs>
          <radialGradient id="sb-glow" cx="50%" cy="50%" r="50%">
            <stop offset="0.6" stopColor="#86efac" stopOpacity="0.45" />
            <stop offset="1" stopColor="#86efac" stopOpacity="0" />
          </radialGradient>
          <linearGradient id="sb-glass" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0" style={{ stopColor: "var(--c-glass-1)" }} />
            <stop offset="1" style={{ stopColor: "var(--c-glass-2)" }} />
          </linearGradient>
          <linearGradient id="sb-ring" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0" stopColor="#4ade80" />
            <stop offset="0.55" stopColor="#16a34a" />
            <stop offset="1" stopColor="#0d9488" />
          </linearGradient>
          <filter id="sb-blur" x="-50%" y="-50%" width="200%" height="200%">
            <feGaussianBlur stdDeviation="14" />
          </filter>
          <filter id="sb-soft" x="-20%" y="-20%" width="140%" height="140%">
            <feGaussianBlur stdDeviation="6" />
          </filter>
        </defs>

        <g className="breathe">
          <circle cx="150" cy="150" r="149" fill="url(#sb-glow)" />
        </g>
        <circle cx="150" cy="150" r="140" fill="url(#sb-glass)" strokeWidth="1.5" style={{ stroke: "var(--c-glass-edge)" }} />

        {/* Light drifting inside the glass. */}
        <g className="spin-slow" filter="url(#sb-blur)">
          <ellipse cx="208" cy="100" rx="62" ry="40" fill="#4ade80" opacity="0.5" />
          <ellipse cx="96" cy="212" rx="54" ry="30" fill="#67e8f9" opacity="0.45" />
          <ellipse cx="110" cy="90" rx="30" ry="22" fill="#c4b5fd" opacity="0.35" />
        </g>

        <circle cx="150" cy="150" r={r} fill="none" strokeWidth="22" style={{ stroke: "var(--c-ring-track)" }} />
        {/* Soft glow under the progress. */}
        <motion.circle
          cx="150"
          cy="150"
          r={r}
          fill="none"
          stroke="#22c55e"
          strokeWidth="22"
          strokeLinecap="round"
          strokeDasharray={c}
          transform="rotate(-90 150 150)"
          filter="url(#sb-soft)"
          opacity={0.45}
          initial={{ strokeDashoffset: c }}
          animate={{ strokeDashoffset: c * (1 - pct / 100) }}
          transition={{ duration: 1.2, ease: [0.2, 0.8, 0.2, 1] }}
        />
        <motion.circle
          cx="150"
          cy="150"
          r={r}
          fill="none"
          stroke="url(#sb-ring)"
          strokeWidth="22"
          strokeLinecap="round"
          strokeDasharray={c}
          transform="rotate(-90 150 150)"
          initial={{ strokeDashoffset: c }}
          animate={{ strokeDashoffset: c * (1 - pct / 100) }}
          transition={{ duration: 1.2, ease: [0.2, 0.8, 0.2, 1] }}
        />
        <circle cx="150" cy="150" r="96" style={{ fill: "var(--c-bg)" }} opacity="0.94" />
        <circle cx="150" cy="150" r="96" fill="none" strokeWidth="1" style={{ stroke: "var(--c-glass-edge)" }} />
      </svg>

      <div className="absolute inset-0 flex flex-col items-center justify-center text-center">
        <HardDrive className="mb-1 size-5 text-muted" strokeWidth={1.75} aria-hidden />
        <div className="text-[30px] font-bold leading-tight tracking-[-0.02em]">{used !== null ? <AnimatedNumber value={used} format={(n) => bytes(n, 0)} /> : "—"}</div>
        <div className="text-[13px] text-muted">used of {total ? bytes(total, 0) : "—"}</div>
        <div className="my-2 h-px w-24 bg-line" />
        <div className="text-[14px] font-semibold">{free !== null ? `${bytes(free, 0)} free` : ""}</div>
        <div className="text-[12px] text-muted">{free !== null && total ? `${Math.round((free / total) * 100)}% free` : ""}</div>
      </div>
    </div>
  );
}
