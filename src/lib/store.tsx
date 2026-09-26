import { createContext, useCallback, useContext, useEffect, useState, type ReactNode } from "react";

import { api, onScanProgress } from "./api";
import { bytes } from "./format";
import type { CleanerScan, DeleteMode, DeleteReport, MemoryInfo, ScanProgress, SecurityReport, Summary } from "./types";

export type Page = "dashboard" | "cleaner" | "files" | "security" | "memory" | "settings";

interface Toast {
  id: number;
  text: string;
  tone: "ok" | "error";
}

interface Store {
  page: Page;
  go: (p: Page) => void;
  home: string | null;
  deleteMode: DeleteMode;
  setDeleteMode: (m: DeleteMode) => void;
  cleaner: CleanerScan | null;
  setCleaner: (s: CleanerScan | null) => void;
  security: SecurityReport | null;
  setSecurity: (s: SecurityReport | null) => void;
  files: Summary | null;
  setFiles: (s: Summary | null) => void;
  /** Live memory, sampled every 2 s for the whole app. */
  mem: MemoryInfo | null;
  /** Last 30 samples (60 s) of RAM used, in percent. */
  memHistory: number[];
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

export function StoreProvider({ children }: { children: ReactNode }) {
  const [page, go] = useState<Page>("dashboard");
  const [home, setHome] = useState<string | null>(null);
  const [deleteMode, setMode] = useState<DeleteMode>(loadMode);
  const [cleaner, setCleaner] = useState<CleanerScan | null>(null);
  const [security, setSecurity] = useState<SecurityReport | null>(null);
  const [files, setFiles] = useState<Summary | null>(null);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [mem, setMem] = useState<MemoryInfo | null>(null);
  const [memHistory, setMemHistory] = useState<number[]>([]);

  useEffect(() => {
    const sample = () =>
      api
        .memoryLive()
        .then((m) => {
          setMem(m);
          const pct = m.total_bytes > 0 ? (m.used_bytes / m.total_bytes) * 100 : 0;
          setMemHistory((h) => [...h.slice(-29), pct]);
        })
        .catch(() => {});
    sample();
    const t = setInterval(sample, 2000);
    return () => clearInterval(t);
  }, []);

  useEffect(() => {
    api.homeDir().then(setHome).catch(() => {});
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
        toast(`Freed ${bytes(r.bytes_freed)} — ${r.removed.length} item${r.removed.length > 1 ? "s" : ""} ${mode === "trash" ? "moved to Trash" : "deleted"}.`);
      }
      if (r.failed.length) {
        toast(`${r.failed.length} item${r.failed.length > 1 ? "s" : ""} couldn't be removed: ${r.failed[0].error}`, "error");
      }
    },
    [toast],
  );

  return (
    <Ctx.Provider value={{ page, go, home, deleteMode, setDeleteMode, cleaner, setCleaner, security, setSecurity, files, setFiles, mem, memHistory, toasts, toast, reportDelete }}>
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
