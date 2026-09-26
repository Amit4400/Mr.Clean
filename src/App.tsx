import { BrushCleaning, Cpu, FolderClosed, House, Settings as SettingsIcon, ShieldCheck, type LucideIcon } from "lucide-react";
import { AnimatePresence, MotionConfig, motion } from "motion/react";
import { useEffect, type ReactNode } from "react";

import { cx } from "./components/ui";
import { StoreProvider, useStore, type Page } from "./lib/store";
import Cleaner from "./pages/Cleaner";
import LargeFiles from "./pages/LargeFiles";
import Memory from "./pages/Memory";
import Overview from "./pages/Overview";
import Security from "./pages/Security";
import Settings from "./pages/Settings";

const NAV: { id: Page; label: string; icon: LucideIcon }[] = [
  { id: "overview", label: "Overview", icon: House },
  { id: "clean", label: "Clean", icon: BrushCleaning },
  { id: "files", label: "Files", icon: FolderClosed },
  { id: "security", label: "Security", icon: ShieldCheck },
  { id: "memory", label: "Memory", icon: Cpu },
];

const isMac = typeof navigator !== "undefined" && /Mac/.test(navigator.platform || navigator.userAgent);

function NavItem({ active, label, icon: Icon, shortcut, extra, onClick }: { active: boolean; label: string; icon: LucideIcon; shortcut?: string; extra?: ReactNode; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      aria-current={active ? "page" : undefined}
      title={shortcut ? `${label} (${shortcut})` : label}
      className={cx(
        "relative flex h-11 w-full cursor-pointer items-center gap-3.5 rounded-[12px] px-3.5 text-[14.5px] transition-colors",
        active ? "font-semibold text-accent-text" : "text-muted hover:bg-ink/[0.04] hover:text-ink",
      )}
    >
      {active && (
        <motion.span
          layoutId="nav-active"
          className="absolute inset-0 rounded-[12px] bg-accent-soft shadow-[inset_0_1px_0_rgba(255,255,255,0.6)] ring-1 ring-accent/15"
          transition={{ type: "spring", stiffness: 500, damping: 40 }}
        />
      )}
      <Icon className="relative size-5" strokeWidth={active ? 2 : 1.75} aria-hidden />
      <span className="relative flex-1 text-left">{label}</span>
      {extra && <span className="relative">{extra}</span>}
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
    overview: <Overview />,
    clean: <Cleaner />,
    files: <LargeFiles />,
    security: <Security />,
    memory: <Memory />,
    settings: <Settings />,
  };
  const mod = isMac ? "⌘" : "Ctrl+";

  return (
    <div className="flex h-full">
      <aside className="flex w-[228px] shrink-0 flex-col px-3 pb-4">
        <div data-tauri-drag-region className="h-11 shrink-0" />
        <div data-tauri-drag-region className="mb-6 flex items-center gap-2.5 px-2">
          <span className="glass flex size-11 shrink-0 items-center justify-center rounded-full">
            <img src="/logo.png" alt="" className="size-8 drop-shadow-[0_3px_8px_rgba(34,197,94,0.35)]" draggable={false} />
          </span>
          <div data-tauri-drag-region>
            <div className="text-[18px] font-bold leading-tight tracking-[-0.01em]">Mr.Clean</div>
            <div className="whitespace-nowrap text-[11px] text-muted">Keep your Mac fast &amp; safe</div>
          </div>
        </div>
        {/* One glass panel holds every destination, Settings included. */}
        <nav aria-label="Main" className="glass flex flex-col gap-1 rounded-[16px] p-1.5">
          {NAV.map((n, i) => (
            <NavItem
              key={n.id}
              active={page === n.id}
              label={n.label}
              icon={n.icon}
              shortcut={`${mod}${i + 1}`}
              onClick={() => go(n.id)}
              extra={
                n.id === "security" && alerts > 0 ? (
                  <span className="rounded-full bg-danger px-1.5 text-[11px] font-semibold leading-[18px] text-white" aria-label={`${alerts} alerts`}>
                    {alerts}
                  </span>
                ) : undefined
              }
            />
          ))}
          <div className="mx-2 my-1 h-px bg-line" />
          <NavItem active={page === "settings"} label="Settings" icon={SettingsIcon} shortcut={`${mod},`} onClick={() => go("settings")} />
        </nav>
      </aside>

      <main className="relative my-2.5 mr-2.5 flex-1 overflow-hidden rounded-[22px] border border-line bg-panel shadow-[0_10px_40px_-12px_rgba(15,23,42,0.18)]">
        {/* Soft colour blobs behind the content. */}
        <div className="pointer-events-none absolute inset-0 overflow-hidden" aria-hidden>
          <div className="blob" style={{ width: 420, height: 420, left: "38%", top: -160, background: "var(--c-blob-a)" }} />
          <div className="blob" style={{ width: 380, height: 380, right: -140, bottom: -120, background: "var(--c-blob-b)", animationDelay: "-9s" }} />
          <div className="blob" style={{ width: 300, height: 300, left: -120, bottom: -60, background: "var(--c-blob-c)", animationDelay: "-17s" }} />
        </div>
        <div className="relative h-full overflow-y-auto">
          <div data-tauri-drag-region className="absolute inset-x-0 top-0 h-6" />
          <AnimatePresence mode="wait" initial={false}>
            <motion.div
              key={page}
              initial={{ opacity: 0, y: 6 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, transition: { duration: 0.08 } }}
              transition={{ duration: 0.2, ease: [0.2, 0.8, 0.2, 1] }}
            >
              {pages[page]}
            </motion.div>
          </AnimatePresence>
        </div>
      </main>

      <div className="pointer-events-none fixed bottom-5 right-5 z-50 flex flex-col items-end gap-2" aria-live="polite">
        <AnimatePresence>
          {toasts.map((t) => (
            <motion.div
              key={t.id}
              initial={{ opacity: 0, y: 12, scale: 0.97 }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              exit={{ opacity: 0, transition: { duration: 0.12 } }}
              transition={{ duration: 0.22, ease: [0.2, 0.8, 0.2, 1] }}
              className={cx("glass max-w-sm rounded-[12px] px-4 py-2.5 text-[13px]", t.tone === "error" && "text-danger-text")}
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
