import { Cpu, FolderSearch, Gauge, Settings as SettingsIcon, ShieldCheck, Sparkles, type LucideIcon } from "lucide-react";
import { AnimatePresence, MotionConfig, motion } from "motion/react";
import { useEffect, type ReactNode } from "react";

import { cx } from "./components/ui";
import { StoreProvider, useStore, type Page } from "./lib/store";
import Cleaner from "./pages/Cleaner";
import Dashboard from "./pages/Dashboard";
import LargeFiles from "./pages/LargeFiles";
import Memory from "./pages/Memory";
import Security from "./pages/Security";
import Settings from "./pages/Settings";

const NAV: { id: Page; label: string; icon: LucideIcon; color: string }[] = [
  { id: "dashboard", label: "Dashboard", icon: Gauge, color: "var(--c-accent)" },
  { id: "cleaner", label: "Dev Cleaner", icon: Sparkles, color: "var(--c-purple)" },
  { id: "files", label: "Large Files", icon: FolderSearch, color: "var(--c-teal)" },
  { id: "security", label: "Security", icon: ShieldCheck, color: "var(--c-safe)" },
  { id: "memory", label: "Memory", icon: Cpu, color: "var(--c-warn)" },
];

const isMac = typeof navigator !== "undefined" && /Mac/.test(navigator.platform || navigator.userAgent);

function NavItem({ active, label, icon: Icon, color, shortcut, extra, onClick }: { active: boolean; label: string; icon: LucideIcon; color: string; shortcut?: string; extra?: ReactNode; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      aria-current={active ? "page" : undefined}
      title={shortcut ? `${label} (${shortcut})` : label}
      className={cx(
        "flex h-[30px] w-full cursor-pointer items-center gap-2 rounded-[6px] px-2 text-[13px] transition-colors",
        active ? "bg-ink/[0.09] font-medium" : "hover:bg-ink/[0.05]",
      )}
    >
      <span className="flex size-[20px] items-center justify-center rounded-[5px] text-white shadow-[0_1px_1px_rgba(0,0,0,0.15)]" style={{ background: color }}>
        <Icon className="size-[13px]" strokeWidth={2.3} aria-hidden />
      </span>
      <span className="flex-1 text-left">{label}</span>
      {extra}
    </button>
  );
}

function Shell() {
  const { page, go, toasts, security } = useStore();
  const alerts = security ? security.counts.high + security.counts.medium : 0;

  // ⌘1–⌘5 (Ctrl on Windows/Linux) switch pages; ⌘, opens Settings.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(isMac ? e.metaKey : e.ctrlKey)) return;
      const n = Number(e.key);
      if (n >= 1 && n <= NAV.length) {
        e.preventDefault();
        go(NAV[n - 1].id);
      } else if (e.key === ",") {
        e.preventDefault();
        go("settings");
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [go]);

  const pages: Record<Page, ReactNode> = {
    dashboard: <Dashboard />,
    cleaner: <Cleaner />,
    files: <LargeFiles />,
    security: <Security />,
    memory: <Memory />,
    settings: <Settings />,
  };
  const mod = isMac ? "⌘" : "Ctrl+";

  return (
    <div className="flex h-full">
      <aside className="sidebar flex w-[216px] shrink-0 flex-col border-r border-line bg-sidebar px-2.5 pb-3">
        <div data-tauri-drag-region className="h-[52px] shrink-0" />
        <nav aria-label="Main" className="flex flex-col gap-px">
          {NAV.map((n, i) => (
            <NavItem
              key={n.id}
              active={page === n.id}
              label={n.label}
              icon={n.icon}
              color={n.color}
              shortcut={`${mod}${i + 1}`}
              onClick={() => go(n.id)}
              extra={
                n.id === "security" && alerts > 0 ? (
                  <span className="rounded-full bg-danger px-1.5 text-[11px] font-semibold leading-[17px] text-white" aria-label={`${alerts} alerts`}>
                    {alerts}
                  </span>
                ) : undefined
              }
            />
          ))}
        </nav>
        <div className="mt-auto">
          <NavItem active={page === "settings"} label="Settings" icon={SettingsIcon} color="#8e8e93" shortcut={`${mod},`} onClick={() => go("settings")} />
        </div>
      </aside>

      <main className="relative flex-1 overflow-y-auto bg-bg">
        <AnimatePresence mode="wait" initial={false}>
          <motion.div
            key={page}
            initial={{ opacity: 0, y: 6 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, transition: { duration: 0.08 } }}
            transition={{ duration: 0.18, ease: [0.2, 0.8, 0.2, 1] }}
          >
            {pages[page]}
          </motion.div>
        </AnimatePresence>
      </main>

      <div className="pointer-events-none fixed bottom-4 right-4 z-50 flex flex-col items-end gap-2" aria-live="polite">
        <AnimatePresence>
          {toasts.map((t) => (
            <motion.div
              key={t.id}
              initial={{ opacity: 0, y: 12, scale: 0.97 }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              exit={{ opacity: 0, transition: { duration: 0.12 } }}
              transition={{ duration: 0.22, ease: [0.2, 0.8, 0.2, 1] }}
              className={cx(
                "max-w-sm rounded-[10px] border px-3.5 py-2.5 text-[13px] shadow-[0_8px_30px_rgba(0,0,0,0.18)] backdrop-blur-xl",
                t.tone === "ok" ? "border-line bg-surface/90 text-ink" : "border-danger/30 bg-danger-soft text-danger-text",
              )}
            >
              {t.text}
            </motion.div>
          ))}
        </AnimatePresence>
      </div>
    </div>
  );
}

export default function App() {
  return (
    <MotionConfig reducedMotion="user">
      <StoreProvider>
        <Shell />
      </StoreProvider>
    </MotionConfig>
  );
}
