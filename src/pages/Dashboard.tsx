import { ArrowRight, Cpu, FolderSearch, HardDrive, ShieldAlert, ShieldCheck, Sparkles } from "lucide-react";
import { useEffect, useState } from "react";

import { Badge, Button, Card, Meter, Ring, Stat, type Tone } from "../components/ui";
import { api } from "../lib/api";
import { bytes, duration, percent } from "../lib/format";
import { useStore } from "../lib/store";
import type { MemoryInfo, SystemInfo } from "../lib/types";

const pressureTone: Record<string, Tone> = { normal: "accent", warning: "warn", critical: "danger" };

export default function Dashboard() {
  const { go, cleaner, setCleaner, security, setSecurity, toast } = useStore();
  const [info, setInfo] = useState<SystemInfo | null>(null);
  const [mem, setMem] = useState<MemoryInfo | null>(null);
  const [scanning, setScanning] = useState(false);

  useEffect(() => {
    api.systemInfo().then((i) => {
      setInfo(i);
      setMem(i.memory);
    });
    const t = setInterval(() => api.memoryLive().then(setMem).catch(() => {}), 2000);
    return () => clearInterval(t);
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
  const diskPct = disk ? percent(used, disk.total_bytes) : 0;
  const diskTone: Tone = diskPct > 90 ? "danger" : diskPct > 80 ? "warn" : "accent";
  const memPct = mem ? percent(mem.used_bytes, mem.total_bytes) : 0;
  const topRules = [...(cleaner?.rules ?? [])].sort((a, b) => b.total_bytes - a.total_bytes).slice(0, 5);
  const threats = security ? security.counts.high + security.counts.medium : null;

  return (
    <div>
      <div className="mb-5 flex items-end justify-between">
        <div>
          <h1 className="text-[22px] font-semibold tracking-tight">{info?.hostname || "Your Mac"}</h1>
          <p className="mt-1 text-sm text-muted">
            {[info?.model, info?.cpu_brand, info?.os_version].filter(Boolean).join(" · ") || "Loading…"}
          </p>
        </div>
        <Button variant="primary" busy={scanning} onClick={scanAll}>
          <Sparkles className="size-4" />
          {scanning ? "Scanning…" : "Scan everything"}
        </Button>
      </div>

      <div className="grid grid-cols-2 gap-4">
        <Card className="flex items-center gap-6 p-5">
          <Ring value={diskPct} tone={diskTone}>
            <div className="tabular text-2xl font-semibold">{diskPct}%</div>
            <div className="text-xs text-muted">used</div>
          </Ring>
          <div className="flex-1 space-y-3">
            <div className="flex items-center gap-2 text-sm font-semibold">
              <HardDrive className="size-4 text-muted" /> Storage
            </div>
            <Stat label="Free" value={disk ? bytes(disk.free_bytes) : "—"} hint={disk ? `of ${bytes(disk.total_bytes)}` : undefined} />
            {diskPct > 85 && <Badge tone={diskTone}>Running low</Badge>}
          </div>
        </Card>

        <Card className="flex items-center gap-6 p-5">
          <Ring value={memPct} tone={pressureTone[mem?.pressure ?? "normal"]}>
            <div className="tabular text-2xl font-semibold">{memPct}%</div>
            <div className="text-xs text-muted">RAM used</div>
          </Ring>
          <div className="flex-1 space-y-3">
            <div className="flex items-center gap-2 text-sm font-semibold">
              <Cpu className="size-4 text-muted" /> Memory
            </div>
            <Stat label="In use" value={mem ? bytes(mem.used_bytes) : "—"} hint={mem ? `of ${bytes(mem.total_bytes)} · swap ${bytes(mem.swap_used_bytes)}` : undefined} />
            {mem && (
              <Badge tone={pressureTone[mem.pressure]}>
                {mem.pressure === "normal" ? "Pressure normal" : mem.pressure === "warning" ? "Memory is tight" : "Memory critical"}
              </Badge>
            )}
          </div>
        </Card>
      </div>

      <div className="mt-4 grid grid-cols-3 gap-4">
        <ActionCard
          icon={<Sparkles className="size-5" />}
          title="Developer junk"
          value={cleaner ? bytes(cleaner.total_bytes) : "Not scanned"}
          hint={cleaner ? `${bytes(cleaner.safe_bytes)} safe to clean right away` : "Xcode, simulators, Gradle, npm…"}
          onClick={() => go("cleaner")}
        />
        <ActionCard
          icon={threats ? <ShieldAlert className="size-5" /> : <ShieldCheck className="size-5" />}
          title="Security"
          value={threats === null ? "Not scanned" : threats === 0 ? "No threats" : `${threats} to review`}
          hint={security ? `${security.scanned_repos} repos · ${security.scanned_packages.toLocaleString()} packages checked` : "Malware, git hooks, npm worms"}
          tone={threats ? "danger" : "accent"}
          onClick={() => go("security")}
        />
        <ActionCard
          icon={<FolderSearch className="size-5" />}
          title="Large files"
          value="Find space hogs"
          hint="See which folders and files use the most space"
          onClick={() => go("files")}
        />
      </div>

      {topRules.length > 0 && (
        <Card className="mt-4 p-5">
          <div className="mb-3 flex items-center justify-between">
            <div className="text-sm font-semibold">Biggest wins</div>
            <div className="flex gap-3 text-xs text-muted">
              <span className="flex items-center gap-1.5"><span className="size-2 rounded-full bg-accent" />Safe</span>
              <span className="flex items-center gap-1.5"><span className="size-2 rounded-full bg-warn" />Review first</span>
              <span className="flex items-center gap-1.5"><span className="size-2 rounded-full bg-faint" />Needs a command</span>
            </div>
          </div>
          <div className="space-y-3">
            {topRules.map((r) => (
              <div key={r.rule.id} className="flex items-center gap-3">
                <div className="w-56 truncate text-sm">{r.rule.name}</div>
                <Meter value={percent(r.total_bytes, topRules[0].total_bytes)} tone={r.rule.safety === "safe" ? "accent" : r.rule.safety === "review" ? "warn" : "neutral"} />
                <div className="tabular w-20 text-right text-sm font-medium">{bytes(r.total_bytes)}</div>
              </div>
            ))}
          </div>
        </Card>
      )}

      {info && (
        <p className="mt-6 text-xs text-faint">
          {info.cpu_cores} CPU cores · up {duration(info.uptime_secs)} · CPU {mem ? Math.round(mem.cpu_percent) : 0}%
        </p>
      )}
    </div>
  );
}

function ActionCard({ icon, title, value, hint, onClick, tone = "accent" }: { icon: React.ReactNode; title: string; value: string; hint: string; onClick: () => void; tone?: Tone }) {
  return (
    <button onClick={onClick} className="group text-left">
      <Card className="h-full p-5 transition group-hover:border-accent">
        <div className={tone === "danger" ? "text-danger" : "text-accent"}>{icon}</div>
        <div className="mt-3 text-xs font-medium text-muted">{title}</div>
        <div className="tabular mt-0.5 text-lg font-semibold">{value}</div>
        <div className="mt-1 flex items-center justify-between gap-2 text-xs text-faint">
          <span>{hint}</span>
          <ArrowRight className="size-4 shrink-0 opacity-0 transition group-hover:opacity-100" />
        </div>
      </Card>
    </button>
  );
}
