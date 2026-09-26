import { Box, BrushCleaning, Check, ChevronRight, Clock, Copy, FolderClosed, Info, Package, ScrollText, Sparkles, StopCircle, Trash2, type LucideIcon } from "lucide-react";
import { motion } from "motion/react";
import { useMemo, useState } from "react";

import { AppIcon } from "../components/AppIcon";
import { AnimatedNumber, Badge, Button, Card, Checkbox, Chip, Empty, Modal, Page, Skeleton, SoftTile, cx, type Tone } from "../components/ui";
import { api } from "../lib/api";
import { ago, bytes, tildify } from "../lib/format";
import { useScanProgress, useStore } from "../lib/store";
import { CHIPS, CHIP_LABEL, chipOf, toolOf, type ChipId } from "../lib/tools";
import type { CleanRequest, NodeModulesHit, RuleScan, Safety } from "../lib/types";

const SAFETY: Record<Safety, { label: string; tone: Tone }> = {
  safe: { label: "Safe", tone: "safe" },
  review: { label: "Review", tone: "warn" },
  report_only: { label: "Info only", tone: "neutral" },
};

// Generic icons for rules that aren't one tool.
const GENERIC: Record<string, LucideIcon> = {
  "user-caches": Box,
  "linux-cache": Box,
  "user-logs": ScrollText,
  "win-temp": Clock,
  trash: Trash2,
};

type Selection = Record<string, Set<string>>;

function defaultSelection(rules: RuleScan[]): Selection {
  const s: Selection = {};
  for (const r of rules) {
    s[r.rule.id] = new Set(r.rule.safety === "safe" ? r.items.map((i) => i.path) : []);
  }
  return s;
}

export default function Cleaner() {
  const { cleaner, setCleaner, home, deleteMode, reportDelete, toast } = useStore();
  const [scanning, setScanning] = useState(false);
  const [sel, setSel] = useState<Selection>(() => (cleaner ? defaultSelection(cleaner.rules) : {}));
  const [open, setOpen] = useState<Set<string>>(new Set());
  const [chip, setChip] = useState<ChipId | "all">("all");
  const [confirm, setConfirm] = useState(false);
  const [cleaning, setCleaning] = useState(false);
  const progress = useScanProgress("cleaner", scanning);

  const scan = async () => {
    setScanning(true);
    try {
      const s = await api.cleanerScan();
      setCleaner(s);
      setSel(defaultSelection(s.rules));
    } catch (e) {
      toast(String(e), "error");
    } finally {
      setScanning(false);
    }
  };

  const selected = useMemo(() => {
    if (!cleaner) return { bytes: 0, count: 0, requests: [] as CleanRequest[], lines: [] as { name: string; bytes: number; permanent: boolean }[] };
    let total = 0;
    let count = 0;
    const requests: CleanRequest[] = [];
    const lines: { name: string; bytes: number; permanent: boolean }[] = [];
    for (const r of cleaner.rules) {
      const set = sel[r.rule.id];
      if (!set || set.size === 0) continue;
      const items = r.items.filter((i) => set.has(i.path));
      const b = items.reduce((a, i) => a + i.bytes, 0);
      total += b;
      count += items.length;
      requests.push({ rule_id: r.rule.id, paths: items.map((i) => i.path) });
      lines.push({ name: r.rule.name, bytes: b, permanent: r.rule.always_permanent });
    }
    return { bytes: total, count, requests, lines };
  }, [cleaner, sel]);

  const clean = async () => {
    setCleaning(true);
    try {
      const report = await api.cleanerClean(selected.requests, deleteMode);
      reportDelete(report, deleteMode);
      setConfirm(false);
      await scan();
    } catch (e) {
      toast(String(e), "error");
    } finally {
      setCleaning(false);
    }
  };

  const toggleRule = (r: RuleScan, on: boolean) => setSel((s) => ({ ...s, [r.rule.id]: new Set(on ? r.items.map((i) => i.path) : []) }));
  const toggleItem = (r: RuleScan, path: string, on: boolean) =>
    setSel((s) => {
      const next = new Set(s[r.rule.id]);
      if (on) next.add(path);
      else next.delete(path);
      return { ...s, [r.rule.id]: next };
    });

  const rules = useMemo(() => [...(cleaner?.rules ?? [])].sort((a, b) => b.total_bytes - a.total_bytes), [cleaner]);
  const chips = useMemo(() => CHIPS.filter((c) => rules.some((r) => chipOf(r.rule.id, r.rule.category) === c)), [rules]);
  const shown = chip === "all" ? rules : rules.filter((r) => chipOf(r.rule.id, r.rule.category) === chip);
  const itemCount = rules.reduce((a, r) => a + r.items.length, 0);

  return (
    <Page
      title="Clean"
      subtitle="Caches and leftovers from Xcode, Android, npm and other developer tools. Your own files are never touched."
      actions={
        scanning ? (
          <Button onClick={() => api.cancelScan()}>
            <StopCircle className="size-3.5" aria-hidden /> Stop
          </Button>
        ) : (
          cleaner && (
            <Button onClick={scan}>
              <Sparkles className="size-3.5" aria-hidden /> Rescan
            </Button>
          )
        )
      }
    >
      <div className="grid grid-cols-[minmax(0,1fr)_300px] items-start gap-5">
        <div className="min-w-0">
          {scanning && (
            <Card className="p-5">
              <p className="mb-3 text-[13px] text-muted" aria-live="polite">
                Measuring caches… <span className="tabular font-medium text-ink">{bytes(progress?.bytes ?? 0)}</span> in{" "}
                <span className="tabular">{(progress?.files ?? 0).toLocaleString()}</span> files so far
              </p>
              <Skeleton rows={6} />
            </Card>
          )}

          {!cleaner && !scanning && (
            <Card>
              <Empty
                icon={<BrushCleaning className="size-7" aria-hidden />}
                title="Find developer junk"
                action={
                  <Button variant="primary" onClick={scan}>
                    Scan
                  </Button>
                }
              >
                Scans Xcode DerivedData, iOS simulators, Android emulators, Gradle, npm/yarn/pnpm, CocoaPods, Homebrew and app caches. Nothing is deleted until you confirm.
              </Empty>
            </Card>
          )}

          {cleaner && !scanning && (
            <>
              {/* Summary: how much is selected, and the one primary action. */}
              <Card className="relative mb-4 overflow-hidden p-5">
                <div className="pointer-events-none absolute -right-10 -top-16 size-48 rounded-full bg-[radial-gradient(circle,rgba(74,222,128,0.35),transparent_65%)]" aria-hidden />
                <div className="relative flex items-center gap-4">
                  <SoftTile icon={BrushCleaning} color="var(--c-accent)" size={58} round />
                  <div className="min-w-0 flex-1">
                    <div className="text-[30px] font-bold leading-none tracking-[-0.02em]">
                      <AnimatedNumber value={selected.bytes} format={bytes} />
                    </div>
                    <div className="mt-1 text-[13px] font-medium text-ink/80">selected to clean</div>
                    <div className="mt-0.5 line-clamp-2 text-[12px] text-muted">
                      {bytes(cleaner.total_bytes)} found in {itemCount.toLocaleString()} items · {bytes(cleaner.safe_bytes)} marked safe ·{" "}
                      {deleteMode === "trash" ? "items go to the Trash" : "items are deleted permanently"}
                    </div>
                  </div>
                  <div className="flex shrink-0 flex-col items-end gap-1.5">
                    <motion.button
                      whileHover={selected.count ? { y: -1 } : undefined}
                      whileTap={selected.count ? { scale: 0.98 } : undefined}
                      transition={{ type: "spring", stiffness: 500, damping: 32 }}
                      disabled={selected.count === 0}
                      onClick={() => setConfirm(true)}
                      className="flex h-11 cursor-pointer items-center gap-2 rounded-[13px] border border-white/40 bg-gradient-to-br from-[#4ade80] to-[#16a34a] px-5 text-[14px] font-semibold text-white shadow-[inset_0_1px_0_rgba(255,255,255,0.45),0_12px_26px_-12px_rgba(22,163,74,0.8)] disabled:cursor-default disabled:opacity-45"
                    >
                      <Trash2 className="size-4" aria-hidden /> Clean selected{selected.count ? ` (${bytes(selected.bytes)})` : ""}
                    </motion.button>
                    <Button variant="ghost" size="sm" onClick={() => setSel(defaultSelection(cleaner.rules))}>
                      Select safe only
                    </Button>
                  </div>
                </div>
                {cleaner.xcode_installed === false && rules.some((r) => r.rule.category === "xcode") && (
                  <div className="relative mt-4 flex items-start gap-2 rounded-[12px] bg-warn-soft px-3 py-2 text-[12.5px]">
                    <Info className="mt-0.5 size-3.5 shrink-0 text-warn-text" aria-hidden />
                    <span>
                      <b>Xcode isn't installed</b>, but its simulators and caches are still on disk. They're leftovers and safe to remove.
                    </span>
                  </div>
                )}
              </Card>

              <div className="mb-3 flex flex-wrap gap-2" role="group" aria-label="Filter by tool">
                <Chip active={chip === "all"} onClick={() => setChip("all")}>
                  All
                </Chip>
                {chips.map((c) => (
                  <Chip key={c} active={chip === c} onClick={() => setChip(c)}>
                    {CHIP_LABEL[c]}
                  </Chip>
                ))}
              </div>

              {rules.length === 0 ? (
                <Card>
                  <Empty icon={<Check className="size-7" aria-hidden />} title="Nothing to clean">
                    No developer caches found. Nice and tidy.
                  </Empty>
                </Card>
              ) : (
                <Card className="divide-y divide-line overflow-hidden">
                  {shown.map((r) => (
                    <RuleRow
                      key={r.rule.id}
                      r={r}
                      home={home}
                      selected={sel[r.rule.id] ?? new Set()}
                      open={open.has(r.rule.id)}
                      onToggleOpen={() =>
                        setOpen((o) => {
                          const n = new Set(o);
                          if (n.has(r.rule.id)) n.delete(r.rule.id);
                          else n.add(r.rule.id);
                          return n;
                        })
                      }
                      onRule={(on) => toggleRule(r, on)}
                      onItem={(p, on) => toggleItem(r, p, on)}
                    />
                  ))}
                </Card>
              )}
            </>
          )}
        </div>

        <NodeModulesPanel />
      </div>

      <Modal
        open={confirm}
        title={`Clean ${bytes(selected.bytes)}?`}
        onClose={() => !cleaning && setConfirm(false)}
        footer={
          <>
            <Button onClick={() => setConfirm(false)} disabled={cleaning}>
              Cancel
            </Button>
            <Button variant="danger" busy={cleaning} onClick={clean}>
              {deleteMode === "trash" ? "Move to Trash" : "Delete permanently"}
            </Button>
          </>
        }
      >
        <p className="mb-3 text-muted">
          {selected.count} item{selected.count === 1 ? "" : "s"} will be{" "}
          {deleteMode === "trash" ? "moved to the Trash. Empty the Trash to get the space back." : "deleted permanently."}
        </p>
        <ul className="space-y-1.5">
          {selected.lines.map((l) => (
            <li key={l.name} className="flex justify-between gap-4">
              <span>
                {l.name}
                {l.permanent && <span className="ml-2 text-xs text-warn">(permanent)</span>}
              </span>
              <span className="tabular font-medium">{bytes(l.bytes)}</span>
            </li>
          ))}
        </ul>
        <p className="mt-4 text-xs text-faint">Quit Xcode, Android Studio and your editors first for the best result.</p>
      </Modal>
    </Page>
  );
}

function RuleIcon({ id }: { id: string }) {
  const tool = toolOf(id);
  if (tool)
    return (
      <span className="glass flex size-9 shrink-0 items-center justify-center rounded-[11px]">
        <AppIcon name={id} tool={tool} size={22} />
      </span>
    );
  return <SoftTile icon={GENERIC[id] ?? Package} color="var(--c-info)" size={36} />;
}

function RuleRow({
  r,
  home,
  selected,
  open,
  onToggleOpen,
  onRule,
  onItem,
}: {
  r: RuleScan;
  home: string | null;
  selected: Set<string>;
  open: boolean;
  onToggleOpen: () => void;
  onRule: (on: boolean) => void;
  onItem: (path: string, on: boolean) => void;
}) {
  const reportOnly = r.rule.safety === "report_only";
  const all = selected.size === r.items.length && r.items.length > 0;
  const [copied, setCopied] = useState(false);
  return (
    <div>
      <div
        className="flex cursor-pointer items-center gap-3 px-4 py-3 transition-colors hover:bg-ink/[0.03]"
        onClick={onToggleOpen}
        role="button"
        tabIndex={0}
        aria-expanded={open}
        onKeyDown={(e) => (e.key === "Enter" || e.key === " ") && (e.preventDefault(), onToggleOpen())}
      >
        <Checkbox label={r.rule.name} checked={all} indeterminate={selected.size > 0} disabled={reportOnly} onChange={onRule} />
        <RuleIcon id={r.rule.id} />
        <div className="min-w-0 flex-1">
          <div className="truncate text-[13.5px] font-semibold">{r.rule.name}</div>
          <div className="truncate text-[12px] text-muted">{r.note && r.rule.category !== "xcode" ? r.note : r.rule.description}</div>
        </div>
        <div className="tabular w-20 text-right text-[13.5px] font-semibold">{bytes(r.total_bytes)}</div>
        <span className="flex w-[72px] justify-center">
          <Badge tone={SAFETY[r.rule.safety].tone}>{SAFETY[r.rule.safety].label}</Badge>
        </span>
        <ChevronRight className={cx("size-4 shrink-0 text-faint transition-transform duration-200", open && "rotate-90")} aria-hidden />
      </div>
      {open && (
        <div className="border-t border-line bg-ink/[0.02] px-4 pb-3 pl-[76px] pt-2.5">
          <p className="mb-2 text-[12px] text-muted">{r.rule.description}</p>
          {r.rule.command && (
            <div className="mb-2 flex items-center gap-2">
              <code className="selectable flex-1 truncate rounded-[8px] border border-line bg-surface px-2 py-1 font-mono text-[11.5px]">{r.rule.command}</code>
              <Button
                size="sm"
                variant="ghost"
                aria-label="Copy command"
                onClick={() => {
                  navigator.clipboard?.writeText(r.rule.command!.split("   #")[0]);
                  setCopied(true);
                  setTimeout(() => setCopied(false), 1500);
                }}
              >
                {copied ? <Check className="size-3.5" /> : <Copy className="size-3.5" />}
              </Button>
            </div>
          )}
          <ul className="divide-y divide-line/60">
            {r.items.slice(0, 200).map((i) => (
              <li key={i.path} className="flex items-center gap-3 py-1.5 text-[12px]">
                <Checkbox label={i.name} checked={selected.has(i.path)} disabled={reportOnly} onChange={(on) => onItem(i.path, on)} />
                <span className="selectable min-w-0 flex-1 truncate font-mono text-[11.5px] text-muted" title={i.path}>
                  {tildify(i.path, home)}
                </span>
                <span className="w-24 text-right text-faint">{ago(i.modified)}</span>
                <span className="tabular w-16 text-right font-medium">{bytes(i.bytes)}</span>
              </li>
            ))}
          </ul>
          {r.items.length > 200 && <div className="pt-1 text-xs text-faint">…and {r.items.length - 200} more</div>}
        </div>
      )}
    </div>
  );
}

const baseName = (p: string) => p.split("/").filter(Boolean).pop() ?? p;
const parentOf = (p: string) => p.slice(0, p.replace(/\/+$/, "").lastIndexOf("/")) || "/";

const AGES = [
  { value: 30, label: "30 days" },
  { value: 90, label: "90 days" },
  { value: 180, label: "6 months" },
];

/** Side panel: node_modules folders in projects you haven't touched lately. */
function NodeModulesPanel() {
  const { home, deleteMode, reportDelete, toast, nodeModules: hits, setNodeModules: setHits } = useStore();
  const [busy, setBusy] = useState(false);
  const [days, setDays] = useState(90);
  const [sel, setSel] = useState<Set<string>>(new Set());
  const [removing, setRemoving] = useState(false);

  const find = async (d = days) => {
    setBusy(true);
    try {
      setHits(await api.nodeModulesFind(d));
      setSel(new Set());
    } catch (e) {
      toast(String(e), "error");
    } finally {
      setBusy(false);
    }
  };
  const remove = async () => {
    setRemoving(true);
    try {
      const r = await api.nodeModulesRemove([...sel], deleteMode);
      reportDelete(r, deleteMode);
      const gone = new Set(r.removed.map((x) => x.path));
      setHits(hits?.filter((x) => !gone.has(x.path)) ?? null);
      setSel(new Set());
    } finally {
      setRemoving(false);
    }
  };
  // Online-only iCloud copies use no space here, so there's nothing to free.
  const list: NodeModulesHit[] = (hits ?? []).filter((h) => !h.in_cloud && h.bytes > 0);
  const inCloud = (hits ?? []).filter((h) => h.in_cloud).length;
  const selBytes = list.filter((h) => sel.has(h.path)).reduce((a, h) => a + h.bytes, 0);
  const total = list.reduce((a, h) => a + h.bytes, 0);

  return (
    <Card className="sticky top-4 p-4">
      <div className="mb-1 flex items-start justify-between gap-2">
        <h2 className="text-[15px] font-semibold">Old node_modules</h2>
        <select
          aria-label="Untouched for"
          className="glass h-7 cursor-pointer rounded-[8px] px-1.5 text-[12px]"
          value={days}
          onChange={(e) => {
            const d = Number(e.target.value);
            setDays(d);
            if (hits) find(d);
          }}
        >
          {AGES.map((a) => (
            <option key={a.value} value={a.value}>
              {a.label}
            </option>
          ))}
        </select>
      </div>
      <p className="mb-3 text-[12px] text-muted">Projects you haven't touched in a while. `npm install` brings them back when you need them.</p>

      {!hits ? (
        <Button className="w-full" busy={busy} onClick={() => find()}>
          <Package className="size-3.5" aria-hidden /> Find old node_modules
        </Button>
      ) : list.length === 0 ? (
        <p className="py-4 text-center text-[12.5px] text-muted">
          {inCloud ? "Nothing to free: your node_modules folders are stored in iCloud, not on this Mac." : "No node_modules folders found in your projects."}
        </p>
      ) : (
        <>
          <div className="mb-1 flex items-center justify-between text-[11.5px] text-faint">
            <span>
              {list.length} projects · {bytes(total)}
            </span>
            <button className="cursor-pointer font-medium text-accent-text hover:underline" onClick={() => setSel(new Set(list.filter((h) => h.stale).map((h) => h.path)))}>
              Select old ones
            </button>
          </div>
          <ul className="-mx-1 max-h-[340px] overflow-y-auto">
            {list.map((h) => (
              <li key={h.path} className="flex items-center gap-2.5 rounded-[10px] px-1 py-2 hover:bg-ink/[0.03]">
                <Checkbox
                  label={h.project}
                  checked={sel.has(h.path)}
                  onChange={(on) =>
                    setSel((s) => {
                      const n = new Set(s);
                      if (on) n.add(h.path);
                      else n.delete(h.path);
                      return n;
                    })
                  }
                />
                <FolderClosed className="size-4 shrink-0 fill-info/20 text-info" aria-hidden />
                <div className="min-w-0 flex-1">
                  <div className="selectable truncate text-[12.5px] font-semibold" title={h.path}>
                    {baseName(h.project)}
                  </div>
                  <div className="selectable truncate text-[11px] text-faint" title={h.project}>
                    {tildify(parentOf(h.project), home)}
                  </div>
                  <div className="text-[11px] text-faint">
                    {ago(h.last_touched)}
                    {h.stale && <span className="text-warn-text"> · old</span>}
                  </div>
                </div>
                <span className="tabular text-[12.5px] font-semibold">{bytes(h.bytes)}</span>
              </li>
            ))}
          </ul>
          {inCloud > 0 && (
            <p className="mt-2 text-[11px] text-faint">
              {inCloud} more project{inCloud === 1 ? " is" : "s are"} stored in iCloud and {inCloud === 1 ? "doesn't" : "don't"} use space on this Mac.
            </p>
          )}
          <Button variant="danger" className="mt-3 w-full" disabled={sel.size === 0} busy={removing} onClick={remove}>
            <Trash2 className="size-3.5" aria-hidden /> Remove {sel.size > 0 ? bytes(selBytes) : "selected"}
          </Button>
        </>
      )}
    </Card>
  );
}
