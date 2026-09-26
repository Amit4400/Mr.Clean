import { Cpu, FolderSearch, Gauge, Settings as SettingsIcon, ShieldCheck, Sparkles } from "lucide-react";
import type { ReactNode } from "react";

import { cx } from "./components/ui";
import { StoreProvider, useStore, type Page } from "./lib/store";
import Cleaner from "./pages/Cleaner";
import Dashboard from "./pages/Dashboard";
import LargeFiles from "./pages/LargeFiles";
import Memory from "./pages/Memory";
import Security from "./pages/Security";
import Settings from "./pages/Settings";

const NAV: { id: Page; label: string; icon: ReactNode }[] = [
  { id: "dashboard", label: "Dashboard", icon: <Gauge className="size-4" /> },
  { id: "cleaner", label: "Dev Cleaner", icon: <Sparkles className="size-4" /> },
  { id: "files", label: "Large Files", icon: <FolderSearch className="size-4" /> },
  { id: "security", label: "Security", icon: <ShieldCheck className="size-4" /> },
  { id: "memory", label: "Memory", icon: <Cpu className="size-4" /> },
];

function Shell() {
  const { page, go, toasts, security } = useStore();
  const alerts = security ? security.counts.high + security.counts.medium : 0;
  const pages: Record<Page, ReactNode> = {
    dashboard: <Dashboard />,
    cleaner: <Cleaner />,
    files: <LargeFiles />,
    security: <Security />,
    memory: <Memory />,
    settings: <Settings />,
  };
  const item = (id: Page, label: string, icon: ReactNode, extra?: ReactNode) => (
    <button
      key={id}
      onClick={() => go(id)}
      className={cx(
        "flex h-9 w-full items-center gap-2.5 rounded-lg px-3 text-sm font-medium transition",
        page === id ? "bg-accent-soft text-accent" : "text-muted hover:bg-surface-2 hover:text-ink",
      )}
    >
      {icon}
      <span className="flex-1 text-left">{label}</span>
      {extra}
    </button>
  );

  return (
    <div className="flex h-full">
      <aside className="flex w-56 shrink-0 flex-col border-r border-line bg-surface px-3 pb-3">
        <div data-tauri-drag-region className="h-10 shrink-0" />
        <div data-tauri-drag-region className="mb-5 flex items-center gap-2 px-2">
          <div className="flex size-8 items-center justify-center rounded-lg bg-accent text-accent-ink">
            <Sparkles className="size-4" />
          </div>
          <div>
            <div className="text-[15px] font-semibold leading-tight">Mr.Clean</div>
            <div className="text-[11px] text-faint">for developer Macs</div>
          </div>
        </div>
        <nav className="flex flex-col gap-0.5">
          {NAV.map((n) =>
            item(
              n.id,
              n.label,
              n.icon,
              n.id === "security" && alerts > 0 ? (
                <span className="rounded-full bg-danger px-1.5 text-[11px] font-semibold text-white">{alerts}</span>
              ) : undefined,
            ),
          )}
        </nav>
        <div className="mt-auto">{item("settings", "Settings", <SettingsIcon className="size-4" />)}</div>
      </aside>
      <main className="relative flex-1 overflow-y-auto">
        <div data-tauri-drag-region className="sticky top-0 z-10 h-6" />
        <div className="mx-auto max-w-5xl px-8 pb-10">{pages[page]}</div>
      </main>
      <div className="pointer-events-none fixed bottom-4 right-4 z-50 flex flex-col gap-2">
        {toasts.map((t) => (
          <div
            key={t.id}
            className={cx(
              "max-w-sm rounded-lg px-4 py-2.5 text-sm font-medium shadow-lg",
              t.tone === "ok" ? "bg-ink text-bg" : "bg-danger text-white",
            )}
          >
            {t.text}
          </div>
        ))}
      </div>
    </div>
  );
}

export default function App() {
  return (
    <StoreProvider>
      <Shell />
    </StoreProvider>
  );
}
