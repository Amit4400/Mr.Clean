import { Cpu, HardDriveDownload, Info, Lock, Power, RotateCw } from "lucide-react";
import { motion } from "motion/react";
import { useCallback, useEffect, useState } from "react";

import { AppIcon } from "../components/AppIcon";
import { AnimatedNumber, AreaChart, Button, Card, Donut, IconTile, Page, toneFill, type Tone } from "../components/ui";
import { api } from "../lib/api";
import { bytes, percent } from "../lib/format";
import { MEM_HISTORY, MEM_SAMPLE_MS, useStore } from "../lib/store";
import type { AppGroup, DevKind, MemoryBreakdown, MemorySnapshot, ProcInfo } from "../lib/types";

const DEV_LABEL: Record<DevKind, string> = {
  gradle_daemon: "Gradle daemon",
  kotlin_daemon: "Kotlin daemon",
  adb_server: "ADB server",
  android_emulator: "Android emulator",
  ios_simulator: "iOS Simulator",
  dev_server: "Dev server",
  orphan_node: "Orphaned Node process",
  docker: "Docker",
  language_server: "Editor helper",
};
/** Brand logo for each kind of developer leftover. */
const DEV_TOOL: Record<DevKind, string> = {
  gradle_daemon: "gradle",
  kotlin_daemon: "kotlin",
  adb_server: "android",
  android_emulator: "androidstudio",
  ios_simulator: "xcode",
  dev_server: "nodedotjs",
  orphan_node: "nodedotjs",
  docker: "docker",
  language_server: "nodedotjs",
};
const pressureTone: Record<string, Tone> = { normal: "safe", warning: "warn", critical: "danger" };
const pressureText: Record<string, string> = { normal: "Normal", warning: "Tight", critical: "Critical" };

const PARTS = [
  { key: "apps", label: "Apps", color: "#3b82f6" },
  { key: "wired", label: "Wired", color: "#8b5cf6" },
  { key: "compressed", label: "Compressed", color: "#f59e0b" },
  { key: "free", label: "Free", color: "#22c55e" },
] as const;

export default function Memory() {
  const { toast, mem, memHistory } = useStore();
  const [snap, setSnap] = useState<MemorySnapshot | null>(null);
  const [parts, setParts] = useState<MemoryBreakdown | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [failed, setFailed] = useState<Set<string>>(new Set());

  const refresh = useCallback(() => {
    api.memorySnapshot().then(setSnap).catch(() => {});
    api.memoryBreakdown().then(setParts).catch(() => {});
  }, []);

  useEffect(() => {
    refresh();
    const t = setInterval(refresh, 4000);
    return () => clearInterval(t);
  }, [refresh]);

  const after = (key: string, ok: boolean, message: string) => {
    toast(message, ok ? "ok" : "error");
    setFailed((f) => {
      const n = new Set(f);
      if (ok) n.delete(key);
      else n.add(key);
      return n;
    });
    setTimeout(refresh, 800);
  };

  const quitProc = async (p: ProcInfo, force: boolean) => {
    const key = `p${p.pid}`;
    setBusy(key);
    try {
      const r = await api.processQuit(p.pid, p.name, force);
      after(key, r.ok, `${p.name}: ${r.message}`);
    } finally {
      setBusy(null);
    }
  };

  const quitApp = async (g: AppGroup, force: boolean) => {
    const key = `a${g.app}`;
    setBusy(key);
    try {
      const rs = await api.appQuit(g.app, force);
      const ok = rs.every((r) => r.ok);
      after(key, ok, `${g.app}: ${rs.find((r) => !r.ok)?.message ?? rs[0]?.message ?? "Done."}`);
    } finally {
      setBusy(null);
    }
  };

  const used = parts ? parts.apps + parts.wired + parts.compressed : null;
  const pressure = mem?.pressure ?? "normal";
  const minutes = Math.round((MEM_HISTORY * MEM_SAMPLE_MS) / 60000);
  const apps = (snap?.apps ?? []).slice(0, 8);
  const top = apps[0]?.memory_bytes ?? 1;
  const devTotal = (snap?.dev_leftovers ?? []).reduce((a, p) => a + p.memory_bytes, 0);

  return (
    <Page
      title="Memory"
      subtitle="See what's using your RAM right now, and what you can safely close."
      actions={
        <Button onClick={refresh}>
          <RotateCw className="size-3.5" aria-hidden /> Refresh
        </Button>
      }
    >
      <div className="mb-5 grid grid-cols-[minmax(0,1.25fr)_minmax(0,1.4fr)_190px] gap-4">
        <Card className="p-4">
          <h2 className="mb-3 text-[14px] font-semibold">Memory usage</h2>
          <div className="flex items-center gap-5">
            <Donut
              size={136}
              stroke={15}
              label="Memory usage"
              parts={PARTS.map((p) => ({ label: p.label, value: parts?.[p.key] ?? 0, color: p.color }))}
            >
              <div className="text-[22px] font-bold leading-tight">{used !== null ? <AnimatedNumber value={used} format={bytes} /> : "—"}</div>
              <div className="text-[11.5px] text-muted">of {parts ? bytes(parts.total, 0) : "—"}</div>
            </Donut>
            <ul className="flex-1 space-y-2 text-[12.5px]">
              {PARTS.map((p) => (
                <li key={p.key} className="flex items-center gap-2">
                  <span className="size-2.5 rounded-full" style={{ background: p.color }} />
                  <span className="flex-1 whitespace-nowrap text-muted">{p.label}</span>
                  <span className="tabular whitespace-nowrap font-semibold">{parts ? bytes(parts[p.key]) : "—"}</span>
                </li>
              ))}
            </ul>
          </div>
        </Card>

        <Card className="flex flex-col p-4">
          <div className="mb-2 flex items-center justify-between">
            <h2 className="text-[14px] font-semibold">Memory pressure</h2>
            <span className="rounded-full border border-line bg-surface px-2.5 py-0.5 text-[11.5px] text-muted">Last {minutes} minutes</span>
          </div>
          <div className="flex-1 pr-10">
            <AreaChart values={memHistory.length ? memHistory : [0]} color={toneFill[pressureTone[pressure]]} height={112} label={`Memory used over the last ${minutes} minutes`} yLabels={["High", "Normal"]} />
          </div>
          <div className="mt-2 flex justify-center">
            <span className="flex items-center gap-1.5 rounded-full bg-surface px-3 py-1 text-[12px] font-medium">
              <span className="size-2 rounded-full" style={{ background: toneFill[pressureTone[pressure]] }} />
              Pressure: {pressureText[pressure]}
            </span>
          </div>
        </Card>

        <div className="flex flex-col gap-4">
          <Card className="flex flex-1 items-center gap-3 p-4">
            <IconTile color="var(--c-info)" size={40}>
              <HardDriveDownload className="size-5" aria-hidden />
            </IconTile>
            <div>
              <div className="text-[12px] text-muted">Swap used</div>
              <div className="tabular text-[20px] font-bold">{mem ? bytes(mem.swap_used_bytes) : "—"}</div>
            </div>
          </Card>
          <Card className="flex flex-1 items-center gap-3 p-4">
            <IconTile color="var(--c-purple)" size={40}>
              <Cpu className="size-5" aria-hidden />
            </IconTile>
            <div>
              <div className="text-[12px] text-muted">CPU usage</div>
              <div className="tabular text-[20px] font-bold">{mem ? `${Math.round(mem.cpu_percent)}%` : "—"}</div>
            </div>
          </Card>
        </div>
      </div>

      <Card className="mb-5 p-4">
        <h2 className="mb-2 px-1 text-[14px] font-semibold">Apps using the most memory</h2>
        <ul className="divide-y divide-line">
          {!snap && <li className="p-4 text-center text-muted">Loading…</li>}
          {apps.map((g, i) => {
            const key = `a${g.app}`;
            return (
              <motion.li
                key={g.app}
                className="grid grid-cols-[28px_minmax(0,1fr)_110px_minmax(80px,1.2fr)_80px_96px] items-center gap-3 px-1 py-2"
                initial={{ opacity: 0, y: 4 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: Math.min(i, 8) * 0.03 }}
              >
                <AppIcon name={g.app} bundle={g.bundle} size={26} />
                <span className="truncate text-[13.5px] font-medium" title={g.app}>
                  {g.app}
                </span>
                <span className="text-[12px] text-faint">
                  {g.process_count} process{g.process_count > 1 ? "es" : ""}
                </span>
                <div className="h-1.5 overflow-hidden rounded-full bg-surface-2">
                  <motion.div className="h-full rounded-full bg-info" initial={{ width: 0 }} animate={{ width: `${percent(g.memory_bytes, top)}%` }} transition={{ duration: 0.6 }} />
                </div>
                <span className="tabular text-right text-[13px] font-semibold">{bytes(g.memory_bytes)}</span>
                <QuitButtons locked={g.protected} busy={busy === key} failed={failed.has(key)} onQuit={(force) => quitApp(g, force)} />
              </motion.li>
            );
          })}
        </ul>
      </Card>

      <Card className="p-4">
        <div className="mb-2 flex items-baseline justify-between px-1">
          <h2 className="text-[14px] font-semibold">Developer leftovers</h2>
          {devTotal > 0 && <span className="tabular text-[12px] text-faint">{bytes(devTotal)} could be freed</span>}
        </div>
        {snap && snap.dev_leftovers.length === 0 && <p className="px-1 py-3 text-[13px] text-muted">No idle daemons, emulators or dev servers running.</p>}
        <ul className="divide-y divide-line">
          {snap?.dev_leftovers.map((p) => {
            const key = `p${p.pid}`;
            return (
              <li key={p.pid} className="flex items-center gap-3 px-1 py-2.5">
                <AppIcon name={p.name} bundle={p.bundle} tool={p.dev_kind ? DEV_TOOL[p.dev_kind] : undefined} size={30} />
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2 text-[13.5px] font-medium">
                    <span>{p.dev_kind ? DEV_LABEL[p.dev_kind] : p.name}</span>
                    <span className="text-[11.5px] font-normal text-faint">PID {p.pid}</span>
                  </div>
                  <div className="truncate text-[12px] text-muted">{p.advice}</div>
                </div>
                <span className="tabular w-20 text-right text-[13px] font-semibold">{bytes(p.memory_bytes)}</span>
                <QuitButtons locked={p.protected} busy={busy === key} failed={failed.has(key)} onQuit={(force) => quitProc(p, force)} />
              </li>
            );
          })}
        </ul>
      </Card>

      <p className="mt-4 flex gap-2 px-1 text-[12px] text-faint">
        <Info className="mt-0.5 size-3.5 shrink-0" aria-hidden />
        macOS keeps spare RAM filled with cache on purpose and frees it instantly, so "RAM booster" purges only slow your Mac down. Closing things you don't need is the real fix.
      </p>
    </Page>
  );
}

function QuitButtons({ locked, busy, failed, onQuit }: { locked: boolean; busy: boolean; failed: boolean; onQuit: (force: boolean) => void }) {
  if (locked) {
    return (
      <span className="flex w-24 items-center justify-end gap-1 text-[12px] text-faint" title="System or other-user process">
        <Lock className="size-3.5" aria-hidden /> Protected
      </span>
    );
  }
  return (
    <div className="flex w-24 justify-end">
      {failed ? (
        <Button size="sm" variant="danger" busy={busy} onClick={() => onQuit(true)}>
          Force quit
        </Button>
      ) : (
        <Button size="sm" busy={busy} onClick={() => onQuit(false)}>
          <Power className="size-3.5" aria-hidden /> Quit
        </Button>
      )}
    </div>
  );
}
