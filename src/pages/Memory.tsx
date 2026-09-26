import { Cpu, Info, Lock, Power, Wrench } from "lucide-react";
import { useCallback, useEffect, useState } from "react";

import { AnimatedNumber, Badge, Button, Card, Meter, Page, Sparkline, Stat, toneFill, type Tone } from "../components/ui";
import { api } from "../lib/api";
import { bytes, percent } from "../lib/format";
import { useStore } from "../lib/store";
import type { AppGroup, DevKind, MemorySnapshot, ProcInfo } from "../lib/types";

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
const pressureTone: Record<string, Tone> = { normal: "safe", warning: "warn", critical: "danger" };

export default function Memory() {
  const { toast, mem, memHistory } = useStore();
  const [snap, setSnap] = useState<MemorySnapshot | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [failed, setFailed] = useState<Set<string>>(new Set());

  const refresh = useCallback(() => {
    api.memorySnapshot().then(setSnap).catch(() => {});
  }, []);

  useEffect(() => {
    refresh();
    const t = setInterval(refresh, 3000);
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

  const devTotal = (snap?.dev_leftovers ?? []).reduce((a, p) => a + p.memory_bytes, 0);

  return (
    <Page title="Memory" subtitle="What's using your RAM right now, and what you can safely close.">

      <Card className="mb-4 p-4">
        <div className="mb-3 flex items-center justify-between">
          <div className="flex items-center gap-2 text-[13px] font-semibold">
            <Cpu className="size-4 text-muted" aria-hidden /> RAM
          </div>
          {mem && (
            <Badge tone={pressureTone[mem.pressure]}>
              {mem.pressure === "normal" ? "Pressure normal" : mem.pressure === "warning" ? "Memory is tight" : "Memory critical"}
            </Badge>
          )}
        </div>
        <Sparkline
          values={memHistory.length ? memHistory : [0]}
          color={toneFill[pressureTone[mem?.pressure ?? "normal"]]}
          height={64}
          label={`Memory used over the last minute, now ${mem ? percent(mem.used_bytes, mem.total_bytes) : 0}%`}
        />
        <div className="mt-4 grid grid-cols-4 gap-4">
          <Stat label="In use" value={mem ? <AnimatedNumber value={mem.used_bytes} format={bytes} /> : "—"} hint={mem ? `of ${bytes(mem.total_bytes)}` : undefined} />
          <Stat label="Available" value={mem ? bytes(mem.available_bytes) : "—"} hint="ready for apps" />
          <Stat label="Swap used" value={mem ? bytes(mem.swap_used_bytes) : "—"} hint="RAM spilled to disk" />
          <Stat label="CPU" value={mem ? `${Math.round(mem.cpu_percent)}%` : "—"} />
        </div>
      </Card>

      <Card className="mb-6 flex gap-3 bg-info-soft p-3.5 text-[13px]">
        <Info className="mt-0.5 size-4 shrink-0 text-info-text" aria-hidden />
        <p className="text-muted">
          macOS keeps spare RAM filled with cache on purpose and frees it instantly when needed, so "RAM booster" purges only slow your Mac down. The real fix is closing things you
          don't need, especially forgotten developer processes. System processes are locked and can't be closed from here.
        </p>
      </Card>

      <section className="mb-6">
        <div className="mb-2 flex items-baseline justify-between px-1">
          <h2 className="flex items-center gap-2 text-[13px] font-semibold">
            <Wrench className="size-4 text-muted" /> Developer leftovers
          </h2>
          {devTotal > 0 && <span className="tabular text-[11.5px] text-faint">{bytes(devTotal)} could be freed</span>}
        </div>
        <Card className="divide-y divide-line">
          {snap && snap.dev_leftovers.length === 0 && <div className="p-5 text-center text-[13px] text-muted">No idle daemons, emulators or dev servers running. 🎉</div>}
          {!snap && <div className="p-5 text-center text-[13px] text-muted">Loading…</div>}
          {snap?.dev_leftovers.map((p) => {
            const key = `p${p.pid}`;
            return (
              <div key={p.pid} className="flex items-center gap-3 px-4 py-3">
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <span className="font-medium">{p.dev_kind ? DEV_LABEL[p.dev_kind] : p.name}</span>
                    <span className="text-[11.5px] text-faint">PID {p.pid}</span>
                  </div>
                  <div className="text-[11.5px] text-muted">{p.advice}</div>
                  <div className="selectable mt-0.5 truncate font-mono text-[11px] text-faint" title={p.command}>
                    {p.command}
                  </div>
                </div>
                <span className="tabular w-20 text-right text-[13px] font-semibold">{bytes(p.memory_bytes)}</span>
                <QuitButtons locked={p.protected} busy={busy === key} failed={failed.has(key)} onQuit={(force) => quitProc(p, force)} />
              </div>
            );
          })}
        </Card>
      </section>

      <section>
        <h2 className="mb-2 px-1 text-[13px] font-semibold">Apps using the most memory</h2>
        <Card className="divide-y divide-line">
          {snap?.apps.slice(0, 25).map((g) => {
            const key = `a${g.app}`;
            return (
              <div key={g.app} className="flex items-center gap-3 px-4 py-2.5 text-[13px]">
                <span className="w-56 truncate font-medium" title={g.app}>
                  {g.app}
                </span>
                <span className="w-24 text-[11.5px] text-faint">
                  {g.process_count} process{g.process_count > 1 ? "es" : ""}
                </span>
                <div className="flex-1">
                  <Meter value={percent(g.memory_bytes, snap.apps[0].memory_bytes)} tone={g.protected ? "neutral" : "accent"} className="h-1.5" />
                </div>
                <span className="tabular w-20 text-right font-semibold">{bytes(g.memory_bytes)}</span>
                <QuitButtons locked={g.protected} busy={busy === key} failed={failed.has(key)} onQuit={(force) => quitApp(g, force)} />
              </div>
            );
          })}
        </Card>
      </section>
    </Page>
  );
}

function QuitButtons({ locked, busy, failed, onQuit }: { locked: boolean; busy: boolean; failed: boolean; onQuit: (force: boolean) => void }) {
  if (locked) {
    return (
      <span className="flex w-28 items-center justify-end gap-1 text-[11.5px] text-faint" title="System or other-user process">
        <Lock className="size-3.5" /> Protected
      </span>
    );
  }
  return (
    <div className="flex w-28 justify-end">
      {failed ? (
        <Button size="sm" variant="danger" busy={busy} onClick={() => onQuit(true)}>
          Force quit
        </Button>
      ) : (
        <Button size="sm" busy={busy} onClick={() => onQuit(false)}>
          <Power className="size-3.5" /> Quit
        </Button>
      )}
    </div>
  );
}
