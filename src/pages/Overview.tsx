import { ChevronRight, CircleCheck, Code, Cpu, FileText, FolderClosed, Loader2, Lock, Package, Play, RotateCw, ShieldCheck, type LucideIcon } from "lucide-react";
import { motion } from "motion/react";
import { useEffect, useRef, useState, type ReactNode } from "react";

import { DeviceArt } from "../components/DeviceArt";
import { StorageBlob } from "../components/StorageBlob";
import { AnimatedNumber, Button, Card, IconTile, cx } from "../components/ui";
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

export default function Overview() {
  const { go, device, cleaner, security, files, nodeModules, memSnap, mem, scan, scanning, scanEverything } = useStore();
  const [info, setInfo] = useState<SystemInfo | null>(null);
  // Size the storage blob to the space between the four stat bubbles.
  const blobBox = useRef<HTMLDivElement>(null);
  const [blobSize, setBlobSize] = useState(300);
  useEffect(() => {
    const el = blobBox.current;
    if (!el) return;
    const ro = new ResizeObserver(([e]) => setBlobSize(Math.max(210, Math.min(300, e.contentRect.width - 290))));
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
    <div className="mx-auto max-w-[1080px] px-8 pb-8 pt-7">
      {/* Header: greeting + dynamic headline, and the device card. */}
      <div className="mb-4 grid grid-cols-[minmax(0,1fr)_minmax(0,430px)] items-start gap-6">
        <div data-tauri-drag-region>
          <div className="text-[11px] font-semibold uppercase tracking-[0.14em] text-muted">{greeting()}</div>
          <h1 className="mt-1 text-[30px] font-bold leading-tight tracking-[-0.025em]">{h.headline}</h1>
          <p className="mt-1 text-[14px] text-muted">{h.reason}</p>
        </div>
        <Card className="flex min-w-0 items-center gap-4 py-3 pl-4 pr-3">
          <DeviceArt kind={device?.kind ?? "laptop"} width={96} />
          <div className="min-w-0 flex-1">
            <div className="truncate text-[15px] font-semibold">{device?.name ?? "This Mac"}</div>
            <div className="line-clamp-2 text-[12px] text-muted">
              {device ? [device.chip.replace(/^Apple /, ""), bytes(device.memory_bytes, 0).replace(" ", " "), device.os_label].filter(Boolean).join("  •  ") : "Loading…"}
            </div>
          </div>
          <span className={cx("flex shrink-0 items-center gap-1.5 whitespace-nowrap rounded-full px-3 py-1.5 text-[12.5px] font-semibold", LEVEL_STYLE[h.level])}>
            <span className="size-2 rounded-full bg-current" />
            {h.label}
          </span>
        </Card>
      </div>

      <div className="grid grid-cols-[minmax(0,1fr)_410px] gap-6">
        {/* Left: storage blob with the four headline numbers around it. */}
        <div className="flex flex-col">
          <div ref={blobBox} className="relative mx-auto h-[330px] w-full max-w-[600px]">
            <div className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2">
              <StorageBlob used={used} total={disk?.total_bytes ?? null} size={blobSize} />
            </div>
            <Bubble className="left-0 top-4" icon={Code} color="var(--c-accent)" label="Developer caches" value={cleaner ? bytes(cleaner.total_bytes) : "—"} sub={cleaner ? "Can be cleaned" : "Not scanned"} />
            <Bubble className="right-0 top-4" icon={FolderClosed} color="var(--c-info)" label="Large files" value={bigBytes !== null ? bytes(bigBytes) : "—"} sub={bigFiles ? `${bigFiles.length} over 500 MB` : "Not scanned"} />
            <Bubble className="bottom-6 left-0" icon={Cpu} color="var(--c-teal)" label="Memory" value={mem ? bytes(mem.used_bytes) : "—"} sub="In use" />
            <Bubble
              className="bottom-6 right-0"
              icon={ShieldCheck}
              color={threats ? "var(--c-danger)" : "var(--c-safe)"}
              label="Threats"
              value={threats !== null ? String(threats) : "—"}
              sub={threats === null ? "Not scanned" : threats === 0 ? "No issues" : "Need attention"}
            />
          </div>

          <ScanButton
            scanning={scanning}
            scanned={scanned}
            steps={scan}
            junk={cleaner?.total_bytes ?? null}
            onScan={scanEverything}
            onReview={() => go("clean")}
          />
          <p className="mt-2.5 flex items-center justify-center gap-1.5 text-[11.5px] text-faint">
            <Lock className="size-3" aria-hidden /> Everything runs locally on your Mac. Nothing is uploaded.
          </p>
        </div>

        {/* Right: what the last scan found. */}
        <Card className="self-start p-4">
          <div className="mb-2 flex items-baseline justify-between px-1">
            <h2 className="text-[17px] font-semibold">What we found</h2>
            {!scanned && !scanning && <span className="text-[12px] text-faint">Run a scan to fill this in</span>}
          </div>
          <div className="flex flex-col gap-1">
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
            <Found
              icon={FileText}
              color="var(--c-warn)"
              title="Large files"
              detail="Big files and folders in your home folder"
              value={bigBytes !== null ? bytes(bigBytes) : null}
              onClick={() => go("files")}
            />
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
      </div>
    </div>
  );
}

function Bubble({ className, icon: Icon, color, label, value, sub }: { className: string; icon: LucideIcon; color: string; label: string; value: string; sub: string }) {
  return (
    <div className={cx("absolute flex items-center gap-3", className)}>
      <span className="glass flex size-12 items-center justify-center rounded-full" style={{ color }}>
        <Icon className="size-5" strokeWidth={2} aria-hidden />
      </span>
      <div>
        <div className="text-[11.5px] text-muted">{label}</div>
        <div className="tabular text-[17px] font-bold leading-tight">{value}</div>
        <div className="text-[11px] text-faint">{sub}</div>
      </div>
    </div>
  );
}

function Found({ icon: Icon, color, title, detail, value, action = "Review", highlight, onClick }: { icon: LucideIcon; color: string; title: string; detail: string; value: ReactNode | null; action?: string; highlight?: boolean; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      className={cx("group flex w-full cursor-pointer items-center gap-3 rounded-[12px] px-2.5 py-2.5 text-left transition-colors", highlight ? "bg-accent-soft" : "hover:bg-ink/[0.03]")}
    >
      <IconTile color={color} size={36}>
        <Icon className="size-[18px]" strokeWidth={2} aria-hidden />
      </IconTile>
      <span className="min-w-0 flex-1">
        <span className="block truncate text-[13.5px] font-semibold">{title}</span>
        <span className="block truncate text-[11.5px] text-muted">{detail}</span>
      </span>
      <span className="tabular shrink-0 whitespace-nowrap text-[13.5px] font-semibold">{value ?? <span className="font-normal text-faint">—</span>}</span>
      <span className={cx("shrink-0 rounded-[8px] border border-line bg-surface px-2.5 py-1 text-[12px] font-semibold", highlight && "text-accent-text")}>{action}</span>
      <ChevronRight className="size-4 shrink-0 text-faint transition-transform group-hover:translate-x-0.5" aria-hidden />
    </button>
  );
}

const STEP_LABEL: Record<ScanStep, string> = { caches: "Caches", node_modules: "node_modules", files: "Large files", security: "Security" };

function ScanButton({ scanning, scanned, steps, junk, onScan, onReview }: { scanning: boolean; scanned: boolean; steps: Record<ScanStep, string>; junk: number | null; onScan: () => void; onReview: () => void }) {
  const primary = scanned && !scanning;
  return (
    <div className="mx-auto mt-2 flex w-full max-w-[460px] flex-col items-center">
      <motion.button
        whileHover={{ scale: 1.01 }}
        whileTap={{ scale: 0.99 }}
        onClick={primary ? onReview : onScan}
        disabled={scanning}
        className="flex h-[62px] w-full cursor-pointer items-center gap-3.5 rounded-[16px] bg-gradient-to-br from-[#4ade80] to-[#16a34a] px-3.5 text-left text-white shadow-[0_12px_30px_-10px_rgba(22,163,74,0.7)] disabled:cursor-default"
      >
        <span className="flex size-10 shrink-0 items-center justify-center rounded-full bg-white/95 text-[#16a34a]">
          {scanning ? <Loader2 className="size-5 animate-spin" aria-hidden /> : primary ? <ChevronRight className="size-5" strokeWidth={2.5} aria-hidden /> : <Play className="ml-0.5 size-5 fill-current" aria-hidden />}
        </span>
        <span className="min-w-0 flex-1">
          <span className="block text-[16px] font-semibold">{scanning ? "Scanning your Mac…" : primary ? "Review cleanup" : "Scan everything"}</span>
          <span className="block truncate text-[12px] text-white/85">
            {scanning
              ? "Caches, node_modules, large files and security"
              : primary
                ? junk !== null
                  ? `${bytes(junk)} of developer junk ready to review`
                  : "See what can be cleaned"
                : "Find junk, large files, security issues and more"}
          </span>
        </span>
      </motion.button>
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
      {primary && (
        <Button variant="ghost" size="sm" className="mt-1.5" onClick={onScan}>
          <RotateCw className="size-3.5" aria-hidden /> Scan again
        </Button>
      )}
    </div>
  );
}
