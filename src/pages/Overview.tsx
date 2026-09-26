import { BrushCleaning, ChevronRight, CircleCheck, ClipboardList, Code, Cpu, FileText, FolderClosed, Loader2, Lock, Package, Play, RotateCw, ShieldCheck, type LucideIcon } from "lucide-react";
import { motion, type Variants } from "motion/react";
import { useEffect, useRef, useState, type ReactNode } from "react";

import { DeviceArt } from "../components/DeviceArt";
import { StorageOrb } from "../components/StorageOrb";
import { AnimatedNumber, Card, cx } from "../components/ui";
import { api } from "../lib/api";
import { bytes } from "../lib/format";
import { greeting, health } from "../lib/health";
import { useStore, type ScanStep } from "../lib/store";
import type { SystemInfo } from "../lib/types";

const BIG_FILE = 500e6;

const LEVEL_STYLE = {
  good: "bg-safe-soft text-safe-text",
  care: "bg-warn-soft text-warn-text",
  attention: "bg-danger-soft text-danger-text",
} as const;

// One shared entrance: sections rise in together, lightly staggered.
const EASE = [0.2, 0.8, 0.2, 1] as const;
const stagger: Variants = { hidden: {}, show: { transition: { staggerChildren: 0.05 } } };
const rise: Variants = { hidden: { opacity: 0, y: 8 }, show: { opacity: 1, y: 0, transition: { duration: 0.35, ease: EASE } } };
const press = { type: "spring", stiffness: 500, damping: 32 } as const;

export default function Overview() {
  const { go, device, cleaner, security, files, nodeModules, memSnap, mem, scan, scanning, scanEverything } = useStore();
  const [info, setInfo] = useState<SystemInfo | null>(null);

  // Size the orb to the space left between the four stat bubbles.
  const stage = useRef<HTMLDivElement>(null);
  const [orbSize, setOrbSize] = useState(360);
  useEffect(() => {
    const el = stage.current;
    if (!el) return;
    const ro = new ResizeObserver(([e]) => setOrbSize(Math.max(260, Math.min(380, e.contentRect.width - 130, e.contentRect.height))));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  useEffect(() => {
    api.systemInfo().then(setInfo).catch(() => {});
  }, [cleaner]);

  const disk = info?.disk ?? null;
  const used = disk ? disk.total_bytes - disk.free_bytes : null;
  const threats = security ? security.counts.high + security.counts.medium : null;
  const bigFiles = files ? files.top_files.filter((f) => f.bytes >= BIG_FILE) : null;
  const bigBytes = bigFiles?.reduce((a, f) => a + f.bytes, 0) ?? null;
  const staleModules = nodeModules?.filter((h) => h.stale) ?? null;
  const moduleBytes = staleModules?.reduce((a, h) => a + h.bytes, 0) ?? null;
  const leftovers = memSnap?.dev_leftovers ?? null;
  const leftoverBytes = leftovers?.reduce((a, p) => a + p.memory_bytes, 0) ?? null;
  const scanned = cleaner !== null || security !== null;

  const h = health({
    diskUsedPercent: disk && used !== null ? (used / disk.total_bytes) * 100 : null,
    pressure: mem?.pressure ?? null,
    highThreats: security?.counts.high ?? 0,
    mediumThreats: security?.counts.medium ?? 0,
    junkBytes: cleaner?.safe_bytes ?? null,
  });

  return (
    <motion.div className="mx-auto max-w-[1180px] px-8 pb-8 pt-7" variants={stagger} initial="hidden" animate="show">
      {/* Header: greeting + dynamic headline, and the device card. */}
      <motion.div variants={rise} className="mb-2 grid grid-cols-[minmax(0,1fr)_minmax(0,440px)] items-start gap-6">
        <div data-tauri-drag-region className="pt-1">
          <div className="text-[11.5px] font-semibold uppercase tracking-[0.16em] text-muted">{greeting()}</div>
          <h1 className="mt-1.5 text-[34px] font-bold leading-[1.1] tracking-[-0.03em]">{h.headline}</h1>
          <p className="mt-2 text-[15px] text-muted">{h.reason}</p>
        </div>
        <Card className="flex min-w-0 items-center gap-4 py-3 pl-4 pr-3">
          <DeviceArt kind={device?.kind ?? "laptop"} width={96} />
          <div className="min-w-0 flex-1">
            <div className="truncate text-[15px] font-semibold">{device?.name ?? "This Mac"}</div>
            <div className="line-clamp-2 text-[12px] text-muted">
              {device ? [device.chip.replace(/^Apple /, ""), bytes(device.memory_bytes, 0), device.os_label].filter(Boolean).join("  •  ") : "Loading…"}
            </div>
          </div>
          <button
            onClick={() => h.page && go(h.page)}
            disabled={!h.page}
            title={h.page ? "Show me" : undefined}
            className={cx(
              "flex shrink-0 cursor-pointer items-center gap-1.5 whitespace-nowrap rounded-full py-1.5 pl-3 pr-2 text-[12.5px] font-semibold transition-transform active:scale-[0.97] disabled:cursor-default",
              LEVEL_STYLE[h.level],
            )}
          >
            <span className="size-2 rounded-full bg-current" />
            {h.label}
            <ChevronRight className="size-3.5 opacity-70" aria-hidden />
          </button>
        </Card>
      </motion.div>

      <div className="grid grid-cols-[minmax(0,1fr)_440px] gap-6">
        {/* Left: the storage orb with the four headline numbers around it. */}
        <motion.div variants={rise} className="flex flex-col">
          <div ref={stage} className="relative mx-auto h-[390px] w-full max-w-[640px]">
            <div className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2">
              <StorageOrb used={used} total={disk?.total_bytes ?? null} size={orbSize} />
            </div>
            <Bubble className="left-0 top-2" icon={Code} color="var(--c-accent)" label="Developer caches" value={cleaner ? bytes(cleaner.total_bytes) : "—"} sub={cleaner ? "Can be cleaned" : "Not scanned"} />
            <Bubble className="right-0 top-2" icon={FolderClosed} color="var(--c-info)" label="Large files" value={bigBytes !== null ? bytes(bigBytes) : "—"} sub={bigFiles ? `${bigFiles.length} over 500 MB` : "Not scanned"} />
            <Bubble className="bottom-2 left-0" icon={Cpu} color="var(--c-teal)" label="Memory" value={mem ? bytes(mem.used_bytes) : "—"} sub="In use" />
            <Bubble
              className="bottom-2 right-0"
              icon={ShieldCheck}
              color={threats ? "var(--c-danger)" : "var(--c-safe)"}
              label="Threats"
              value={threats !== null ? String(threats) : "—"}
              sub={threats === null ? "Not scanned" : threats === 0 ? "No issues" : "Need attention"}
            />
          </div>

          <ScanButtons scanning={scanning} scanned={scanned} steps={scan} junk={cleaner?.total_bytes ?? null} onScan={scanEverything} onReview={() => go("clean")} />
          <p className="mt-3 flex items-center justify-center gap-1.5 text-[11.5px] text-faint">
            <Lock className="size-3" aria-hidden /> Everything runs locally on your Mac. Nothing is uploaded.
          </p>
        </motion.div>

        {/* Right: what the last scan found. */}
        <motion.div variants={rise} className="self-start">
          <Card className="p-4">
            <div className="mb-2 flex items-baseline justify-between px-1.5">
              <h2 className="text-[18px] font-semibold tracking-[-0.01em]">What we found</h2>
              {scanned || scanning ? (
                <button onClick={() => go("clean")} className="cursor-pointer text-[12.5px] font-medium text-accent-text hover:underline">
                  See all
                </button>
              ) : (
                <span className="text-[12px] text-faint">Run a scan to fill this in</span>
              )}
            </div>
            <div className="flex flex-col gap-0.5">
              <Found
                highlight={!!cleaner && cleaner.total_bytes > 0}
                icon={Code}
                color="var(--c-accent)"
                title="Developer caches"
                detail="Xcode, Android, npm, pip and more"
                value={cleaner ? <AnimatedNumber value={cleaner.total_bytes} format={bytes} /> : null}
                onClick={() => go("clean")}
              />
              <Found
                icon={Package}
                color="var(--c-info)"
                title="Old node_modules"
                detail={staleModules ? `${staleModules.length} project${staleModules.length === 1 ? "" : "s"} not touched in 90 days` : "Not used in a while"}
                value={moduleBytes !== null ? bytes(moduleBytes) : null}
                onClick={() => go("clean")}
              />
              <Found icon={FileText} color="var(--c-warn)" title="Large files" detail="Big files and folders in your home folder" value={bigBytes !== null ? bytes(bigBytes) : null} onClick={() => go("files")} />
              <Found
                icon={Cpu}
                color="var(--c-purple)"
                title="Memory leftovers"
                detail="Idle emulators, dev servers and more"
                value={leftoverBytes === null ? null : leftoverBytes === 0 ? <span className="text-safe-text">None</span> : bytes(leftoverBytes)}
                onClick={() => go("memory")}
              />
              <Found
                icon={ShieldCheck}
                color="var(--c-danger)"
                title="Security checks"
                detail="Malware, risky configs and tokens"
                value={
                  threats === null ? null : threats === 0 ? (
                    <span className="text-safe-text">0 issues</span>
                  ) : (
                    <span className="text-danger-text">
                      {threats} issue{threats > 1 ? "s" : ""}
                    </span>
                  )
                }
                action={threats === 0 ? "View" : "Review"}
                onClick={() => go("security")}
              />
            </div>
          </Card>
        </motion.div>
      </div>

      {/* Shortcuts into each tool. */}
      <motion.div variants={rise} className="mt-6">
        <Card className="@container p-5">
          <div className="mb-4 flex items-start justify-between">
            <div>
              <h2 className="text-[18px] font-semibold tracking-[-0.01em]">Quick actions</h2>
              <p className="text-[12.5px] text-muted">Common tasks to keep your Mac clean and fast.</p>
            </div>
            <button onClick={() => go("clean")} className="flex cursor-pointer items-center gap-0.5 text-[12.5px] font-medium text-accent-text hover:underline">
              View all tools <ChevronRight className="size-3.5" aria-hidden />
            </button>
          </div>
          <div className="grid grid-cols-2 gap-3 @5xl:grid-cols-4">
            <Quick icon={BrushCleaning} color="var(--c-accent)" title="Clean developer caches" detail="Remove Xcode, Android, npm and more." onClick={() => go("clean")} />
            <Quick icon={FolderClosed} color="var(--c-info)" title="Find large files" detail="Discover and remove big files and folders." onClick={() => go("files")} />
            <Quick icon={Cpu} color="var(--c-purple)" title="Free memory" detail="Quit dev processes using RAM." onClick={() => go("memory")} />
            <Quick icon={ShieldCheck} color="var(--c-danger)" title="Run security scan" detail="Check for malware, risky configs and tokens." onClick={() => go("security")} />
          </div>
        </Card>
      </motion.div>
    </motion.div>
  );
}

/** A tinted, duotone icon in a soft rounded tile. */
function Tile({ icon: Icon, color, size, round }: { icon: LucideIcon; color: string; size: number; round?: boolean }) {
  return (
    <span
      className={cx("flex shrink-0 items-center justify-center shadow-[inset_0_1px_0_rgba(255,255,255,0.55)]", round ? "rounded-full" : "rounded-[13px]")}
      style={{
        width: size,
        height: size,
        color,
        background: `linear-gradient(145deg, color-mix(in srgb, ${color} 22%, transparent), color-mix(in srgb, ${color} 8%, transparent))`,
      }}
    >
      <Icon style={{ width: size * 0.46, height: size * 0.46 }} strokeWidth={2} fill="currentColor" fillOpacity={0.18} aria-hidden />
    </span>
  );
}

function Bubble({ className, icon, color, label, value, sub }: { className: string; icon: LucideIcon; color: string; label: string; value: string; sub: string }) {
  return (
    <div className={cx("absolute flex items-center gap-3", className)}>
      <span className="glass flex size-[54px] items-center justify-center rounded-full">
        <Tile icon={icon} color={color} size={42} round />
      </span>
      <div>
        <div className="text-[12px] text-muted">{label}</div>
        <div className="tabular text-[19px] font-bold leading-tight">{value}</div>
        <div className="text-[11.5px] text-faint">{sub}</div>
      </div>
    </div>
  );
}

function Found({ icon, color, title, detail, value, action = "Review", highlight, onClick }: { icon: LucideIcon; color: string; title: string; detail: string; value: ReactNode | null; action?: string; highlight?: boolean; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      className={cx("group flex w-full cursor-pointer items-center gap-3 rounded-[14px] px-2.5 py-3 text-left transition-colors", highlight ? "bg-accent-soft" : "hover:bg-ink/[0.035]")}
    >
      <Tile icon={icon} color={color} size={42} />
      <span className="min-w-0 flex-1">
        <span className="block truncate text-[14px] font-semibold">{title}</span>
        <span className="block truncate text-[12px] text-muted">{detail}</span>
      </span>
      <span className="tabular shrink-0 whitespace-nowrap text-[14px] font-semibold">{value ?? <span className="font-normal text-faint">—</span>}</span>
      <span className={cx("glass shrink-0 rounded-[9px] px-3 py-1.5 text-[12px] font-semibold", highlight && "text-accent-text")}>{action}</span>
      <ChevronRight className="size-4 shrink-0 text-faint transition-transform group-hover:translate-x-0.5" aria-hidden />
    </button>
  );
}

function Quick({ icon, color, title, detail, onClick }: { icon: LucideIcon; color: string; title: string; detail: string; onClick: () => void }) {
  return (
    <motion.button
      whileHover={{ y: -2 }}
      whileTap={{ scale: 0.98 }}
      transition={press}
      onClick={onClick}
      className="flex cursor-pointer items-center gap-3 rounded-[14px] border p-3.5 text-left"
      style={{
        borderColor: `color-mix(in srgb, ${color} 16%, transparent)`,
        background: `linear-gradient(135deg, color-mix(in srgb, ${color} 11%, transparent), color-mix(in srgb, ${color} 3%, transparent))`,
      }}
    >
      <Tile icon={icon} color={color} size={46} />
      <span className="min-w-0 flex-1">
        <span className="block text-[13.5px] font-semibold leading-snug">{title}</span>
        <span className="line-clamp-2 block text-[11.5px] leading-snug text-muted">{detail}</span>
      </span>
      <span className="glass flex size-7 shrink-0 items-center justify-center rounded-full text-muted">
        <ChevronRight className="size-3.5" aria-hidden />
      </span>
    </motion.button>
  );
}

const STEP_LABEL: Record<ScanStep, string> = { caches: "Caches", node_modules: "node_modules", files: "Large files", security: "Security" };

function ScanButtons({ scanning, scanned, steps, junk, onScan, onReview }: { scanning: boolean; scanned: boolean; steps: Record<ScanStep, string>; junk: number | null; onScan: () => void; onReview: () => void }) {
  return (
    <div className="mx-auto mt-1 w-full max-w-[600px]">
      <div className="grid grid-cols-[minmax(0,1.55fr)_minmax(0,1fr)] gap-3">
        <motion.button
          whileHover={scanning ? undefined : { y: -1 }}
          whileTap={scanning ? undefined : { scale: 0.985 }}
          transition={press}
          onClick={onScan}
          disabled={scanning}
          className="flex h-[64px] cursor-pointer items-center gap-3.5 rounded-[16px] border border-white/40 bg-gradient-to-br from-[#4ade80] to-[#16a34a] px-3.5 text-left text-white shadow-[inset_0_1px_0_rgba(255,255,255,0.45),0_14px_30px_-12px_rgba(22,163,74,0.75)] disabled:cursor-default"
        >
          <span className="flex size-10 shrink-0 items-center justify-center rounded-full bg-white/95 text-[#16a34a] shadow-sm">
            {scanning ? <Loader2 className="size-5 animate-spin" aria-hidden /> : scanned ? <RotateCw className="size-[18px]" strokeWidth={2.5} aria-hidden /> : <Play className="ml-0.5 size-5 fill-current" aria-hidden />}
          </span>
          <span className="min-w-0 flex-1">
            <span className="block text-[16px] font-semibold">{scanning ? "Scanning your Mac…" : scanned ? "Scan again" : "Scan everything"}</span>
            <span className="block truncate text-[12px] text-white/85">{scanning ? "Caches, node_modules, large files and security" : "Find junk, large files, security issues and more"}</span>
          </span>
        </motion.button>

        <motion.button whileHover={{ y: -1 }} whileTap={{ scale: 0.985 }} transition={press} onClick={onReview} className="glass flex h-[64px] cursor-pointer items-center gap-3 rounded-[16px] px-3.5 text-left">
          <span className="glass flex size-10 shrink-0 items-center justify-center rounded-full text-muted">
            <ClipboardList className="size-[18px]" aria-hidden />
          </span>
          <span className="min-w-0 flex-1">
            <span className="block text-[15px] font-semibold">Review cleanup</span>
            <span className="block truncate text-[12px] text-muted">{junk !== null ? `${bytes(junk)} ready to review` : "See what can be cleaned"}</span>
          </span>
        </motion.button>
      </div>

      {scanning && (
        <div className="mt-3 flex flex-wrap justify-center gap-x-4 gap-y-1 text-[12px] text-muted" aria-live="polite">
          {(Object.keys(STEP_LABEL) as ScanStep[]).map((s) => (
            <span key={s} className="flex items-center gap-1">
              {steps[s] === "done" ? <CircleCheck className="size-3.5 text-safe" aria-hidden /> : steps[s] === "running" ? <Loader2 className="size-3.5 animate-spin" aria-hidden /> : <span className="size-3.5" />}
              {STEP_LABEL[s]}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}
