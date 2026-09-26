import { HardDrive } from "lucide-react";
import { animate } from "motion/react";
import { useEffect, useRef, useState } from "react";

import { bytes } from "../lib/format";
import { AnimatedNumber } from "./ui";

const C = 200; // viewBox centre
const R = 168; // sphere radius

/**
 * A crescent hugging the inside of the sphere: follows the rim from `start`
 * (degrees, 0 = 12 o'clock, clockwise) for `sweep` degrees, fattest in the
 * middle and tapering to points at both ends, like liquid pooled against glass.
 */
export function crescentPath(start: number, sweep: number, outer: number, width: number, steps = 72): string {
  const pt = (deg: number, r: number) => {
    const a = ((deg - 90) * Math.PI) / 180;
    return `${(C + r * Math.cos(a)).toFixed(2)} ${(C + r * Math.sin(a)).toFixed(2)}`;
  };
  const out: string[] = [];
  const inn: string[] = [];
  for (let i = 0; i <= steps; i++) {
    const t = i / steps;
    const deg = start + sweep * t;
    out.push(pt(deg, outer));
    inn.push(pt(deg, outer - width * Math.pow(Math.sin(Math.PI * t), 0.75)));
  }
  return `M${out.join(" L")} L${inn.reverse().join(" L")} Z`;
}

const reduceMotion = () => typeof window !== "undefined" && window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;

/**
 * The storage orb: a glass sphere with light liquid drifting inside. The green
 * swoosh grows clockwise from the top to the share of the disk that's used.
 */
export function StorageOrb({ used, total, size = 340 }: { used: number | null; total: number | null; size?: number }) {
  const pct = used !== null && total ? Math.min(100, Math.max(0, (used / total) * 100)) : 0;
  const free = used !== null && total ? total - used : null;

  const [shown, setShown] = useState(0);
  const from = useRef(0);
  useEffect(() => {
    if (reduceMotion()) {
      setShown(pct);
      from.current = pct;
      return;
    }
    const c = animate(from.current, pct, { duration: 1.3, ease: [0.2, 0.8, 0.2, 1], onUpdate: (v) => ((from.current = v), setShown(v)) });
    return () => c.stop();
  }, [pct]);

  // Keep a visible sliver even for a nearly empty disk.
  const sweep = Math.max(shown > 0 ? 14 : 0, (shown / 100) * 356);
  const scale = size / 340;

  return (
    <div className="relative" style={{ width: size, height: size }}>
      <svg viewBox="0 0 400 400" width={size} height={size} role="img" aria-label={`Disk ${Math.round(pct)}% used`} className="overflow-visible">
        <defs>
          <radialGradient id="orb-halo" cx="50%" cy="50%" r="50%">
            <stop offset="0.74" stopColor="#6ee7b7" stopOpacity="0.55" />
            <stop offset="1" stopColor="#6ee7b7" stopOpacity="0" />
          </radialGradient>
          <radialGradient id="orb-body" cx="42%" cy="36%" r="68%">
            <stop offset="0" style={{ stopColor: "var(--c-orb-1)" }} />
            <stop offset="0.6" style={{ stopColor: "var(--c-orb-2)" }} />
            <stop offset="1" style={{ stopColor: "var(--c-orb-3)" }} />
          </radialGradient>
          <linearGradient id="orb-used" x1="0.2" y1="0" x2="0.8" y2="1">
            <stop offset="0" stopColor="#86efac" />
            <stop offset="0.45" stopColor="#22c55e" />
            <stop offset="1" stopColor="#0f9f6e" />
          </linearGradient>
          <linearGradient id="orb-shine" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0" stopColor="#ffffff" stopOpacity="0.95" />
            <stop offset="0.5" stopColor="#ffffff" stopOpacity="0" />
          </linearGradient>
          <linearGradient id="orb-aqua" x1="0" y1="1" x2="1" y2="0">
            <stop offset="0" stopColor="#5eead4" />
            <stop offset="1" stopColor="#a7f3d0" stopOpacity="0.4" />
          </linearGradient>
          <linearGradient id="orb-sky" x1="0" y1="0" x2="1" y2="0">
            <stop offset="0" stopColor="#7dd3fc" stopOpacity="0.2" />
            <stop offset="0.5" stopColor="#38bdf8" />
            <stop offset="1" stopColor="#7dd3fc" stopOpacity="0.2" />
          </linearGradient>
          <radialGradient id="orb-edge" cx="50%" cy="50%" r="50%">
            <stop offset="0.8" stopColor="#6ee7b7" stopOpacity="0" />
            <stop offset="1" stopColor="#6ee7b7" stopOpacity="0.45" />
          </radialGradient>
          {/* Dark at the inner edge, bright at the glass: reads as a rounded tube. */}
          <radialGradient id="orb-used-depth" gradientUnits="userSpaceOnUse" cx={C} cy={C} r={R + 4}>
            <stop offset="0.55" stopColor="#0f7a4f" />
            <stop offset="0.8" stopColor="#16a34a" />
            <stop offset="0.93" stopColor="#4ade80" />
            <stop offset="1" stopColor="#bbf7d0" />
          </radialGradient>
          <linearGradient id="orb-core-shine" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0" stopColor="#ffffff" stopOpacity="0.35" />
            <stop offset="0.5" stopColor="#ffffff" stopOpacity="0" />
          </linearGradient>
          <filter id="orb-liquid" x="-20%" y="-20%" width="140%" height="140%">
            <feGaussianBlur stdDeviation="7" />
          </filter>
          <filter id="orb-specular" x="-20%" y="-20%" width="140%" height="140%">
            <feGaussianBlur stdDeviation="2.5" />
          </filter>
          <clipPath id="orb-clip">
            <circle cx={C} cy={C} r={R} />
          </clipPath>
          <filter id="orb-glow" x="-30%" y="-30%" width="160%" height="160%">
            <feGaussianBlur stdDeviation="14" />
          </filter>
          <filter id="orb-lift" x="-30%" y="-30%" width="160%" height="160%">
            <feDropShadow dx="0" dy="10" stdDeviation="14" floodColor="#0f766e" floodOpacity="0.22" />
          </filter>
        </defs>

        <g className="breathe">
          <circle cx={C} cy={C} r={200} fill="url(#orb-halo)" />
        </g>
        <circle cx={C} cy={C} r={R} fill="url(#orb-body)" />
        <circle cx={C} cy={C} r={R} fill="url(#orb-edge)" />

        <g clipPath="url(#orb-clip)">
          {/* Soft liquid light pooled against the glass. */}
          <g filter="url(#orb-liquid)">
            <path d={crescentPath(188, 118, R + 4, 74)} fill="url(#orb-aqua)" opacity="0.8" />
            <path d={crescentPath(146, 74, R + 4, 40)} fill="url(#orb-sky)" opacity="0.75" />
            <path d={crescentPath(300, 56, R + 4, 30)} fill="#c4b5fd" opacity="0.4" />
          </g>
          {/* Used space: a rounded, lit swoosh. */}
          {sweep > 0 && (
            <>
              <path d={crescentPath(0, sweep, R + 4, 80)} fill="#22c55e" opacity="0.5" filter="url(#orb-glow)" />
              <path d={crescentPath(0, sweep, R + 4, 74)} fill="url(#orb-used-depth)" />
              <path d={crescentPath(0, sweep, R + 4, 74)} fill="url(#orb-used)" opacity="0.55" />
              <path d={crescentPath(sweep * 0.06, sweep * 0.55, R - 8, 14)} fill="#ffffff" opacity="0.5" filter="url(#orb-specular)" />
            </>
          )}
        </g>

        {/* Glass rim, reflections and the frosted core that holds the numbers. */}
        <circle cx={C} cy={C} r={R + 1.5} fill="none" strokeWidth="1" stroke="#10b981" strokeOpacity="0.25" />
        <circle cx={C} cy={C} r={R} fill="none" strokeWidth="2.5" style={{ stroke: "var(--c-orb-rim)" }} />
        <path d={crescentPath(-80, 74, R - 5, 18)} fill="url(#orb-shine)" filter="url(#orb-specular)" />
        <path d={crescentPath(148, 64, R - 4, 8)} fill="#ffffff" opacity="0.55" filter="url(#orb-specular)" />

        <circle cx={C} cy={C} r={116} style={{ fill: "var(--c-orb-core)" }} filter="url(#orb-lift)" />
        <circle cx={C} cy={C} r={116} fill="url(#orb-core-shine)" />
        <circle cx={C} cy={C} r={116} fill="none" strokeWidth="1.5" style={{ stroke: "var(--c-orb-rim)" }} />
      </svg>

      <div className="absolute inset-0 flex flex-col items-center justify-center text-center" style={{ fontSize: 13 * scale }}>
        <HardDrive className="mb-1 text-muted" style={{ width: 22 * scale, height: 22 * scale }} strokeWidth={1.75} aria-hidden />
        <div className="font-bold leading-tight tracking-[-0.02em]" style={{ fontSize: 34 * scale }}>
          {used !== null ? <AnimatedNumber value={used} format={(n) => bytes(n, 0)} /> : "—"}
        </div>
        <div className="text-muted" style={{ fontSize: 14 * scale }}>
          used of {total ? bytes(total, 0) : "—"}
        </div>
        <div className="my-2 h-px bg-line" style={{ width: 110 * scale }} />
        <div className="font-semibold" style={{ fontSize: 15 * scale }}>
          {free !== null ? `${bytes(free, 0)} free` : ""}
        </div>
        <div className="text-muted" style={{ fontSize: 12 * scale }}>
          {free !== null && total ? `${Math.round((free / total) * 100)}% free` : ""}
        </div>
      </div>
    </div>
  );
}
