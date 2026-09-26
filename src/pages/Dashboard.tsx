import { ChevronRight, Cpu, FolderSearch, HardDrive, ShieldAlert, ShieldCheck, Sparkles, type LucideIcon } from "lucide-react";
import { motion } from "motion/react";
import { useEffect, useState, type ReactNode } from "react";

import { AnimatedNumber, Badge, Button, Card, Group, Meter, Page, Sparkline, Stat, StorageBar, toneFill, type Tone } from "../components/ui";
import { api } from "../lib/api";
import { bytes, duration, percent } from "../lib/format";
import { useStore } from "../lib/store";
import type { SystemInfo } from "../lib/types";

const pressureTone: Record<string, Tone> = { normal: "safe", warning: "warn", critical: "danger" };
const pressureText: Record<string, string> = { normal: "Normal", warning: "Tight", critical: "Critical" };

export default function Dashboard() {
  const { go, cleaner, setCleaner, security, setSecurity, mem, memHistory, toast } = useStore();
  const [info, setInfo] = useState<SystemInfo | null>(null);
  const [scanning, setScanning] = useState(false);

  useEffect(() => {
    api.systemInfo().then(setInfo).catch(() => {});
  }, []);

  const scanAll = async () => {
    setScanning(true);
    try {
      const [c, s] = await Promise.all([api.cleanerScan(), api.securityScan()]);
      setCleaner(c);
      setSecurity(s);
      api.systemInfo().then(setInfo);
    } catch (e) {
      toast(String(e), "error");
    } finally {
      setScanning(false);
    }
  };

  const disk = info?.disk;
  const used = disk ? disk.total_bytes - disk.free_bytes : 0;
  const junk = Math.min(cleaner?.total_bytes ?? 0, used);
  const diskPct = disk ? percent(used, disk.total_bytes) : 0;
  const threats = security ? security.counts.high + security.counts.medium : null;
  const topRules = [...(cleaner?.rules ?? [])].sort((a, b) => b.total_bytes - a.total_bytes).slice(0, 5);
  const pressure = mem?.pressure ?? "normal";

  return (
    <Page
      title="Dashboard"
      actions={
        <Button variant="primary" busy={scanning} onClick={scanAll}>
          {!scanning && <Sparkles className="size-3.5" aria-hidden />}
          {scanning ? "Scanning…" : "Scan everything"}
        </Button>
      }
    >
      <div className="mb-6 flex items-center gap-4">
        <div className="flex size-14 items-center justify-center rounded-[14px] bg-gradient-to-b from-[#6e6e73] to-[#3a3a3c] text-white shadow-md">
          <HardDrive className="size-7" strokeWidth={1.6} aria-hidden />
        </div>
        <div className="min-w-0">
          <div className="truncate text-[20px] font-semibold tracking-[-0.02em]">{info?.hostname || "This Mac"}</div>
          <div className="truncate text-[12.5px] text-muted">
            {[info?.model, info?.cpu_brand, info?.os_version].filter(Boolean).join(" · ") || "Loading…"}
          </div>
        </div>
      </div>

      <Group title="Storage" aside={disk ? `${bytes(disk.free_bytes)} available of ${bytes(disk.total_bytes)}` : undefined}>
        <Card className="p-4">
          {disk ? (
            <>
              <StorageBar
                total={disk.total_bytes}
                format={bytes}
                segments={[
                  ...(junk > 0 ? [{ label: "Developer junk", value: junk, color: "var(--c-warn)" }] : []),
                  { label: junk > 0 ? "Everything else" : "Used", value: used - junk, color: "var(--c-accent)" },
                  { label: "Free", value: disk.free_bytes, color: "color-mix(in srgb, var(--c-faint) 35%, transparent)" },
                ]}
              />
              {diskPct >= 85 && (
                <p className="mt-3 flex items-center gap-2 text-[12.5px] text-muted">
                  <Badge tone={diskPct >= 92 ? "danger" : "warn"}>{diskPct}% full</Badge>
                  Macs slow down when the disk is nearly full. {cleaner ? "Clean developer junk to get space back." : "Run a scan to see what you can free."}
                </p>
              )}
            </>
          ) : (
            <div className="skeleton h-[22px]" />
          )}
        </Card>
      </Group>

      <Group title="Memory" aside={mem ? `${bytes(mem.used_bytes)} of ${bytes(mem.total_bytes)} used` : undefined}>
        <Card className="p-4">
          <div className="grid grid-cols-[1fr_auto] items-end gap-6">
            <div>
              <div className="mb-1 flex items-center justify-between text-[11.5px] text-muted">
                <span>Memory used · last minute</span>
                <span className="flex items-center gap-1.5">
                  <span className="size-2 rounded-full" style={{ background: toneFill[pressureTone[pressure]] }} />
                  Pressure: {pressureText[pressure]}
                </span>
              </div>
              <Sparkline values={memHistory.length ? memHistory : [0]} color={toneFill[pressureTone[pressure]]} label={`Memory used over the last minute, now ${(memHistory[memHistory.length - 1] ?? 0).toFixed(0)}%`} />
            </div>
            <div className="grid grid-cols-3 gap-6 pb-1">
              <Stat label="In use" value={mem ? <AnimatedNumber value={mem.used_bytes} format={bytes} /> : "—"} />
              <Stat label="Swap" value={mem ? bytes(mem.swap_used_bytes) : "—"} />
              <Stat label="CPU" value={mem ? `${Math.round(mem.cpu_percent)}%` : "—"} />
            </div>
          </div>
        </Card>
      </Group>

      <Group title="Tools">
        <Card className="divide-y divide-line overflow-hidden">
          <NavRow
            icon={Sparkles}
            color="var(--c-purple)"
            title="Developer junk"
            detail={cleaner ? `${bytes(cleaner.safe_bytes)} safe to clean right away` : "Xcode, simulators, Gradle, npm and more"}
            value={cleaner ? <AnimatedNumber value={cleaner.total_bytes} format={bytes} /> : <span className="text-faint">Not scanned</span>}
            onClick={() => go("cleaner")}
          />
          <NavRow
            icon={threats ? ShieldAlert : ShieldCheck}
            color={threats ? "var(--c-danger)" : "var(--c-safe)"}
            title="Security"
            detail={security ? `${security.scanned_repos} repos · ${security.scanned_packages.toLocaleString()} packages checked` : "Malware, git hooks, npm worms, exposed tokens"}
            value={
              threats === null ? (
                <span className="text-faint">Not scanned</span>
              ) : threats === 0 ? (
                <Badge tone="safe">No threats</Badge>
              ) : (
                <Badge tone="danger">{threats} to review</Badge>
              )
            }
            onClick={() => go("security")}
          />
          <NavRow icon={FolderSearch} color="var(--c-teal)" title="Large files" detail="See which folders and files use the most space" onClick={() => go("files")} />
          <NavRow icon={Cpu} color="var(--c-warn)" title="Memory" detail="Quit idle Gradle daemons, emulators and dev servers" onClick={() => go("memory")} />
        </Card>
      </Group>

      {topRules.length > 0 && (
        <Group title="Biggest wins" aside={<Legend />}>
          <Card className="space-y-3 p-4">
            {topRules.map((r, i) => (
              <motion.div
                key={r.rule.id}
                className="grid grid-cols-[200px_1fr_72px] items-center gap-3"
                initial={{ opacity: 0, y: 4 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: i * 0.04, duration: 0.25 }}
              >
                <span className="truncate text-[13px]">{r.rule.name}</span>
                <Meter value={percent(r.total_bytes, topRules[0].total_bytes)} tone={r.rule.safety === "safe" ? "safe" : r.rule.safety === "review" ? "warn" : "neutral"} />
                <span className="tabular text-right text-[13px] font-medium">{bytes(r.total_bytes)}</span>
              </motion.div>
            ))}
          </Card>
        </Group>
      )}

      {info && (
        <p className="px-1 text-[11.5px] text-faint">
          {info.cpu_cores} CPU cores · up {duration(info.uptime_secs)}
        </p>
      )}
    </Page>
  );
}

function Legend() {
  const dot = (c: string, t: string) => (
    <span className="flex items-center gap-1.5">
      <span className="size-2 rounded-full" style={{ background: c }} />
      {t}
    </span>
  );
  return (
    <span className="flex gap-3">
      {dot("var(--c-safe)", "Safe")}
      {dot("var(--c-warn)", "Review first")}
      {dot("var(--c-faint)", "Needs a command")}
    </span>
  );
}

function NavRow({ icon: Icon, color, title, detail, value, onClick }: { icon: LucideIcon; color: string; title: string; detail: string; value?: ReactNode; onClick: () => void }) {
  return (
    <button onClick={onClick} className="flex w-full cursor-pointer items-center gap-3 px-4 py-2.5 text-left transition-colors hover:bg-surface-2">
      <span className="flex size-7 items-center justify-center rounded-[7px] text-white shadow-[0_1px_1px_rgba(0,0,0,0.15)]" style={{ background: color }}>
        <Icon className="size-4" strokeWidth={2.2} aria-hidden />
      </span>
      <span className="min-w-0 flex-1">
        <span className="block text-[13px] font-medium">{title}</span>
        <span className="block truncate text-[11.5px] text-muted">{detail}</span>
      </span>
      {value && <span className="text-[13px] font-medium">{value}</span>}
      <ChevronRight className="size-4 text-faint" aria-hidden />
    </button>
  );
}
