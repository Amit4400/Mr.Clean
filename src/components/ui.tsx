import { Loader2 } from "lucide-react";
import { AnimatePresence, animate, motion, useReducedMotion } from "motion/react";
import { useEffect, useRef, useState, type ButtonHTMLAttributes, type ReactNode } from "react";

export function cx(...c: (string | false | null | undefined)[]) {
  return c.filter(Boolean).join(" ");
}

export type Tone = "neutral" | "accent" | "safe" | "danger" | "warn" | "info";

/** Solid fill colour for each tone (bars, rings, dots). */
export const toneFill: Record<Tone, string> = {
  neutral: "var(--c-faint)",
  accent: "var(--c-accent)",
  safe: "var(--c-safe)",
  warn: "var(--c-warn)",
  danger: "var(--c-danger)",
  info: "var(--c-info)",
};

// ------------------------------------------------------------------ layout

/** A page: sticky unified toolbar (title + actions) and padded content. */
export function Page({ title, subtitle, actions, children }: { title: string; subtitle?: ReactNode; actions?: ReactNode; children: ReactNode }) {
  return (
    <div className="min-h-full">
      <header
        data-tauri-drag-region
        className="sticky top-0 z-20 flex h-[52px] items-center gap-3 border-b border-line bg-bg/85 px-7 backdrop-blur-xl"
      >
        <h1 data-tauri-drag-region className="flex-1 truncate text-[15px] font-semibold tracking-[-0.01em]">
          {title}
        </h1>
        {actions && <div className="flex shrink-0 items-center gap-2">{actions}</div>}
      </header>
      <div className="mx-auto max-w-[980px] px-7 pb-12 pt-5">
        {subtitle && <p className="mb-5 max-w-[68ch] text-[13px] text-muted">{subtitle}</p>}
        {children}
      </div>
    </div>
  );
}

/** Grouped inset box, like macOS System Settings. */
export function Card({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <div className={cx("rounded-[10px] border border-line bg-surface shadow-[0_1px_2px_rgba(0,0,0,0.04)]", className)}>
      {children}
    </div>
  );
}

/** A titled section of grouped content. */
export function Group({ title, aside, footer, children, className }: { title?: ReactNode; aside?: ReactNode; footer?: ReactNode; children: ReactNode; className?: string }) {
  return (
    <section className={cx("mb-6", className)}>
      {(title || aside) && (
        <div className="mb-1.5 flex items-baseline justify-between gap-3 px-1">
          {title && <h2 className="text-[13px] font-semibold">{title}</h2>}
          {aside && <div className="text-[12px] text-faint tabular">{aside}</div>}
        </div>
      )}
      {children}
      {footer && <p className="mt-1.5 px-1 text-[11.5px] text-faint">{footer}</p>}
    </section>
  );
}

// ---------------------------------------------------------------- controls

type Variant = "primary" | "secondary" | "danger" | "ghost";
const variants: Record<Variant, string> = {
  primary: "bg-accent text-accent-ink shadow-[0_1px_1px_rgba(0,0,0,0.12)] hover:brightness-105 active:brightness-95",
  secondary:
    "bg-surface text-ink border border-line shadow-[0_1px_1px_rgba(0,0,0,0.06)] hover:bg-surface-2 active:brightness-95",
  danger: "bg-danger text-white shadow-[0_1px_1px_rgba(0,0,0,0.12)] hover:brightness-105 active:brightness-95",
  ghost: "text-accent-text hover:bg-accent-soft active:brightness-95",
};

export function Button({
  variant = "secondary",
  busy,
  children,
  className,
  size = "md",
  ...rest
}: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: Variant; busy?: boolean; size?: "sm" | "md" }) {
  return (
    <button
      {...rest}
      disabled={rest.disabled || busy}
      className={cx(
        "inline-flex shrink-0 cursor-pointer items-center justify-center gap-1.5 rounded-[6px] font-medium transition-[filter,background-color] duration-150 disabled:pointer-events-none disabled:opacity-45",
        size === "sm" ? "h-6 px-2 text-[12px]" : "h-7 px-3 text-[13px]",
        variants[variant],
        className,
      )}
    >
      {busy && <Loader2 className="size-3.5 animate-spin" aria-hidden />}
      {children}
    </button>
  );
}

const badgeTones: Record<Tone, string> = {
  neutral: "bg-surface-2 text-muted",
  accent: "bg-accent-soft text-accent-text",
  safe: "bg-safe-soft text-safe-text",
  warn: "bg-warn-soft text-warn-text",
  danger: "bg-danger-soft text-danger-text",
  info: "bg-info-soft text-info-text",
};

export function Badge({ tone = "neutral", children }: { tone?: Tone; children: ReactNode }) {
  return (
    <span className={cx("inline-flex shrink-0 items-center gap-1 rounded-full px-2 py-px text-[11px] font-medium", badgeTones[tone])}>
      {children}
    </span>
  );
}

export function Checkbox({ checked, indeterminate, onChange, disabled, label }: { checked: boolean; indeterminate?: boolean; onChange: (v: boolean) => void; disabled?: boolean; label?: string }) {
  return (
    <input
      type="checkbox"
      aria-label={label}
      className="size-[15px] shrink-0 cursor-pointer accent-[var(--c-accent)] disabled:cursor-default disabled:opacity-35"
      checked={checked}
      disabled={disabled}
      ref={(el) => {
        if (el) el.indeterminate = !!indeterminate && !checked;
      }}
      onChange={(e) => onChange(e.target.checked)}
      onClick={(e) => e.stopPropagation()}
    />
  );
}

/** macOS segmented control. */
export function Segmented<T extends string | number>({ options, value, onChange, label }: { options: { value: T; label: ReactNode }[]; value: T; onChange: (v: T) => void; label: string }) {
  return (
    <div role="radiogroup" aria-label={label} className="inline-flex rounded-[7px] bg-surface-2 p-[2px] ring-1 ring-line">
      {options.map((o) => {
        const on = o.value === value;
        return (
          <button
            key={String(o.value)}
            role="radio"
            aria-checked={on}
            onClick={() => onChange(o.value)}
            className={cx(
              "relative h-6 cursor-pointer rounded-[5px] px-3 text-[12px] font-medium transition-colors",
              on ? "text-ink" : "text-muted hover:text-ink",
            )}
          >
            {on && (
              <motion.span
                layoutId={`seg-${label}`}
                className="absolute inset-0 rounded-[5px] bg-surface shadow-[0_1px_2px_rgba(0,0,0,0.12)]"
                transition={{ type: "spring", stiffness: 500, damping: 38 }}
              />
            )}
            <span className="relative">{o.label}</span>
          </button>
        );
      })}
    </div>
  );
}

// ------------------------------------------------------------------ charts

export function Meter({ value, tone = "accent", className }: { value: number; tone?: Tone; className?: string }) {
  return (
    <div className={cx("h-1.5 w-full overflow-hidden rounded-full bg-surface-2", className)}>
      <motion.div
        className="h-full rounded-full"
        style={{ background: toneFill[tone] }}
        initial={{ width: 0 }}
        animate={{ width: `${Math.max(0, Math.min(100, value))}%` }}
        transition={{ duration: 0.5, ease: [0.2, 0.8, 0.2, 1] }}
      />
    </div>
  );
}

export function Ring({ value, size = 112, stroke = 10, tone = "accent", children, label }: { value: number; size?: number; stroke?: number; tone?: Tone; children?: ReactNode; label?: string }) {
  const r = (size - stroke) / 2;
  const c = 2 * Math.PI * r;
  const v = Math.max(0, Math.min(100, value));
  return (
    <div className="relative shrink-0" style={{ width: size, height: size }} role="img" aria-label={label ?? `${Math.round(v)}%`}>
      <svg width={size} height={size} className="-rotate-90">
        <circle cx={size / 2} cy={size / 2} r={r} fill="none" stroke="var(--c-surface-2)" strokeWidth={stroke} />
        <motion.circle
          cx={size / 2}
          cy={size / 2}
          r={r}
          fill="none"
          stroke={toneFill[tone]}
          strokeWidth={stroke}
          strokeLinecap="round"
          strokeDasharray={c}
          initial={{ strokeDashoffset: c }}
          animate={{ strokeDashoffset: c * (1 - v / 100) }}
          transition={{ duration: 0.8, ease: [0.2, 0.8, 0.2, 1] }}
        />
      </svg>
      <div className="absolute inset-0 flex flex-col items-center justify-center">{children}</div>
    </div>
  );
}

export interface Segment {
  label: string;
  value: number;
  color: string;
}

/** Horizontal stacked bar, like "About This Mac → Storage". */
export function StorageBar({ segments, total, format }: { segments: Segment[]; total: number; format: (n: number) => string }) {
  const [hover, setHover] = useState<string | null>(null);
  return (
    <div>
      <div className="flex h-[22px] gap-[2px] overflow-hidden rounded-[6px] bg-surface-2" role="img" aria-label={segments.map((s) => `${s.label} ${format(s.value)}`).join(", ")}>
        {segments.map((s, i) => (
          <motion.div
            key={s.label}
            title={`${s.label}: ${format(s.value)}`}
            onMouseEnter={() => setHover(s.label)}
            onMouseLeave={() => setHover(null)}
            className="h-full first:rounded-l-[6px] transition-opacity"
            style={{ background: s.color, opacity: hover && hover !== s.label ? 0.45 : 1 }}
            initial={{ width: 0 }}
            animate={{ width: `${total > 0 ? (s.value / total) * 100 : 0}%` }}
            transition={{ duration: 0.7, delay: i * 0.06, ease: [0.2, 0.8, 0.2, 1] }}
          />
        ))}
      </div>
      <div className="mt-2.5 flex flex-wrap gap-x-4 gap-y-1 text-[12px] text-muted">
        {segments.map((s) => (
          <span key={s.label} className="flex items-center gap-1.5">
            <span className="size-2 rounded-full" style={{ background: s.color }} />
            {s.label} <span className="tabular text-faint">{format(s.value)}</span>
          </span>
        ))}
      </div>
    </div>
  );
}

/** Small area chart (0–100 values), like Activity Monitor's memory pressure. */
export function Sparkline({ values, color, height = 56, label }: { values: number[]; color: string; height?: number; label: string }) {
  const w = 300;
  const pts = values.length > 1 ? values : [values[0] ?? 0, values[0] ?? 0];
  const step = w / (pts.length - 1);
  const y = (v: number) => height - (Math.max(0, Math.min(100, v)) / 100) * (height - 2) - 1;
  const line = pts.map((v, i) => `${i === 0 ? "M" : "L"}${(i * step).toFixed(1)},${y(v).toFixed(1)}`).join(" ");
  return (
    <svg viewBox={`0 0 ${w} ${height}`} preserveAspectRatio="none" className="w-full" style={{ height }} role="img" aria-label={label}>
      <path d={`${line} L${w},${height} L0,${height} Z`} fill={color} opacity={0.18} />
      <path d={line} fill="none" stroke={color} strokeWidth={1.5} vectorEffect="non-scaling-stroke" strokeLinejoin="round" />
    </svg>
  );
}

/** Number that counts up/down smoothly when it changes. */
export function AnimatedNumber({ value, format }: { value: number; format: (n: number) => string }) {
  const reduce = useReducedMotion();
  const [shown, setShown] = useState(value);
  const from = useRef(value);
  useEffect(() => {
    if (reduce) {
      setShown(value);
      return;
    }
    const controls = animate(from.current, value, {
      duration: 0.6,
      ease: [0.2, 0.8, 0.2, 1],
      onUpdate: (v) => setShown(v),
    });
    from.current = value;
    return () => controls.stop();
  }, [value, reduce]);
  return <span className="tabular">{format(shown)}</span>;
}

// ------------------------------------------------------------ misc pieces

export function Stat({ label, value, hint }: { label: string; value: ReactNode; hint?: ReactNode }) {
  return (
    <div>
      <div className="text-[11.5px] text-muted">{label}</div>
      <div className="tabular mt-0.5 text-[17px] font-semibold tracking-[-0.01em]">{value}</div>
      {hint && <div className="text-[11.5px] text-faint">{hint}</div>}
    </div>
  );
}

export function Empty({ icon, title, children, action }: { icon: ReactNode; title: string; children?: ReactNode; action?: ReactNode }) {
  return (
    <div className="flex flex-col items-center px-6 py-16 text-center">
      <div className="mb-4 flex size-14 items-center justify-center rounded-2xl bg-accent-soft text-accent">{icon}</div>
      <h3 className="text-[15px] font-semibold">{title}</h3>
      <div className="mt-1.5 max-w-[46ch] text-[13px] text-muted">{children}</div>
      {action && <div className="mt-5">{action}</div>}
    </div>
  );
}

export function Skeleton({ rows = 4 }: { rows?: number }) {
  return (
    <Card className="divide-y divide-line">
      {Array.from({ length: rows }, (_, i) => (
        <div key={i} className="flex items-center gap-3 px-4 py-3">
          <div className="skeleton size-4" />
          <div className="skeleton h-3 flex-1" style={{ maxWidth: `${60 - i * 8}%` }} />
          <div className="skeleton h-3 w-14" />
        </div>
      ))}
    </Card>
  );
}

/** macOS-style sheet: drops down from the toolbar. Escape closes it. */
export function Modal({ open, title, children, footer, onClose }: { open: boolean; title: string; children: ReactNode; footer: ReactNode; onClose: () => void }) {
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);
  return (
    <AnimatePresence>
      {open && (
        <motion.div
          className="fixed inset-0 z-50 flex justify-center bg-black/25 px-6 pt-[52px]"
          onClick={onClose}
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.15 }}
        >
          <motion.div
            role="dialog"
            aria-modal="true"
            aria-label={title}
            className="h-fit w-full max-w-[460px] overflow-hidden rounded-b-[12px] rounded-t-[4px] border border-line bg-surface shadow-[0_18px_50px_rgba(0,0,0,0.28)]"
            onClick={(e) => e.stopPropagation()}
            initial={{ y: -24, opacity: 0 }}
            animate={{ y: 0, opacity: 1 }}
            exit={{ y: -16, opacity: 0 }}
            transition={{ duration: 0.22, ease: [0.2, 0.8, 0.2, 1] }}
          >
            <div className="px-5 pb-1 pt-5 text-[15px] font-semibold">{title}</div>
            <div className="max-h-[55vh] overflow-auto px-5 py-2 text-[13px]">{children}</div>
            <div className="flex justify-end gap-2 px-5 pb-4 pt-3">{footer}</div>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
