import { createContext, useCallback, useContext, useEffect, useState, type ReactNode } from "react";

import { api, inTauri, onScanProgress } from "./api";
import { bytes } from "./format";
import type {
  CleanerScan,
  DeleteMode,
  DeleteReport,
  DeviceInfo,
  MemoryInfo,
  MemorySnapshot,
  NodeModulesHit,
  ScanProgress,
  SecurityReport,
  Summary,
} from "./types";
import { words } from "./platform";

export type Page = "overview" | "clean" | "files" | "security" | "memory" | "settings";

/** Steps of "Scan everything"; each shows its own status. */
export type ScanStep = "caches" | "node_modules" | "files" | "security";
export type StepState = "idle" | "running" | "done" | "error";

interface Toast {
  id: number;
  text: string;
  tone: "ok" | "error";
}

/** 10 minutes of memory samples, one every 5 s. */
export const MEM_SAMPLE_MS = 5000;
export const MEM_HISTORY = 120;

interface Store {
  page: Page;
  go: (p: Page) => void;
  home: string | null;
  device: DeviceInfo | null;
  deleteMode: DeleteMode;
  setDeleteMode: (m: DeleteMode) => void;
  cleaner: CleanerScan | null;
  setCleaner: (s: CleanerScan | null) => void;
  security: SecurityReport | null;
  setSecurity: (s: SecurityReport | null) => void;
  files: Summary | null;
  setFiles: (s: Summary | null) => void;
  nodeModules: NodeModulesHit[] | null;
  setNodeModules: (s: NodeModulesHit[] | null) => void;
  memSnap: MemorySnapshot | null;
  /** Live memory, refreshed every 2 s. */
  mem: MemoryInfo | null;
  /** RAM used in percent: the last 10 minutes. */
  memHistory: number[];
  scan: Record<ScanStep, StepState>;
  scanning: boolean;
  scanEverything: () => Promise<void>;
  toasts: Toast[];
  toast: (text: string, tone?: "ok" | "error") => void;
  reportDelete: (r: DeleteReport, mode: DeleteMode) => void;
}

const Ctx = createContext<Store | null>(null);

function loadMode(): DeleteMode {
  try {
    return localStorage.getItem("deleteMode") === "permanent" ? "permanent" : "trash";
  } catch {
    return "trash";
  }
}

const IDLE: Record<ScanStep, StepState> = { caches: "idle", node_modules: "idle", files: "idle", security: "idle" };

export function StoreProvider({ children }: { children: ReactNode }) {
  const [page, go] = useState<Page>("overview");
  const [home, setHome] = useState<string | null>(null);
  const [device, setDevice] = useState<DeviceInfo | null>(null);
  const [deleteMode, setMode] = useState<DeleteMode>(loadMode);
  const [cleaner, setCleaner] = useState<CleanerScan | null>(null);
  const [security, setSecurity] = useState<SecurityReport | null>(null);
  const [files, setFiles] = useState<Summary | null>(null);
  const [nodeModules, setNodeModules] = useState<NodeModulesHit[] | null>(null);
  const [memSnap, setMemSnap] = useState<MemorySnapshot | null>(null);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [mem, setMem] = useState<MemoryInfo | null>(null);
  // The browser preview starts with sample history so the chart isn't empty.
  const [memHistory, setMemHistory] = useState<number[]>(() =>
    inTauri ? [] : Array.from({ length: MEM_HISTORY }, (_, i) => 38 + 4 * Math.sin(i / 7) + 2 * Math.sin(i / 2.3)),
  );
  const [scan, setScan] = useState(IDLE);

  // Live numbers every 2 s; one history point every 5 s.
  useEffect(() => {
    let last = 0;
    const sample = () =>
      api
        .memoryLive()
        .then((m) => {
          setMem(m);
          const now = Date.now();
          if (now - last >= MEM_SAMPLE_MS - 100) {
            last = now;
            const pct = m.total_bytes > 0 ? (m.used_bytes / m.total_bytes) * 100 : 0;
            setMemHistory((h) => [...h.slice(-(MEM_HISTORY - 1)), pct]);
          }
        })
        .catch(() => {});
    sample();
    const t = setInterval(sample, 2000);
    return () => clearInterval(t);
  }, []);

  useEffect(() => {
    api.homeDir().then(setHome).catch(() => {});
    api.deviceInfo().then(setDevice).catch(() => {});
    api.memorySnapshot().then(setMemSnap).catch(() => {});
  }, []);

  // Pause background animation while the window isn't visible.
  useEffect(() => {
    const onVis = () => document.documentElement.classList.toggle("paused", document.hidden);
    document.addEventListener("visibilitychange", onVis);
    return () => document.removeEventListener("visibilitychange", onVis);
  }, []);

  const setDeleteMode = (m: DeleteMode) => {
    setMode(m);
    try {
      localStorage.setItem("deleteMode", m);
    } catch {
      /* private mode */
    }
  };

  const toast = useCallback((text: string, tone: "ok" | "error" = "ok") => {
    const id = Date.now() + Math.random();
    setToasts((t) => [...t, { id, text, tone }]);
    setTimeout(() => setToasts((t) => t.filter((x) => x.id !== id)), 5000);
  }, []);

  const reportDelete = useCallback(
    (r: DeleteReport, mode: DeleteMode) => {
      if (r.removed.length) {
        toast(`Freed ${bytes(r.bytes_freed)} — ${r.removed.length} item${r.removed.length > 1 ? "s" : ""} ${mode === "trash" ? `moved to ${words.trash}` : "deleted"}.`);
      }
      if (r.failed.length) {
        toast(`${r.failed.length} item${r.failed.length > 1 ? "s" : ""} couldn't be removed: ${r.failed[0].error}`, "error");
      }
    },
    [toast],
  );

  const scanning = Object.values(scan).includes("running");

  const scanEverything = useCallback(async () => {
    setScan({ caches: "running", node_modules: "running", files: "running", security: "running" });
    const step = async <T,>(name: ScanStep, run: () => Promise<T>, save: (v: T) => void) => {
      try {
        save(await run());
        setScan((s) => ({ ...s, [name]: "done" }));
      } catch (e) {
        setScan((s) => ({ ...s, [name]: "error" }));
        toast(String(e), "error");
      }
    };
    await Promise.all([
      step("caches", api.cleanerScan, setCleaner),
      step("node_modules", () => api.nodeModulesFind(90), setNodeModules),
      step("files", () => api.bigfilesScan(), setFiles),
      step("security", api.securityScan, setSecurity),
      api.memorySnapshot().then(setMemSnap).catch(() => {}),
    ]);
  }, [toast]);

  return (
    <Ctx.Provider
      value={{
        page,
        go,
        home,
        device,
        deleteMode,
        setDeleteMode,
        cleaner,
        setCleaner,
        security,
        setSecurity,
        files,
        setFiles,
        nodeModules,
        setNodeModules,
        memSnap,
        mem,
        memHistory,
        scan,
        scanning,
        scanEverything,
        toasts,
        toast,
        reportDelete,
      }}
    >
      {children}
    </Ctx.Provider>
  );
}

export function useStore(): Store {
  const s = useContext(Ctx);
  if (!s) throw new Error("useStore outside provider");
  return s;
}

/** Live file/byte counters for a running scan. */
export function useScanProgress(task: string, active: boolean): ScanProgress | null {
  const [p, setP] = useState<ScanProgress | null>(null);
  useEffect(() => {
    if (!active) {
      setP(null);
      return;
    }
    let off: (() => void) | undefined;
    onScanProgress((e) => e.task === task && setP(e)).then((u) => (off = u));
    return () => off?.();
  }, [task, active]);
  return p;
}
