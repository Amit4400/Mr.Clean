import { Check, ChevronRight, Copy, Info, Package, Sparkles, StopCircle, Trash2 } from "lucide-react";
import { useMemo, useState } from "react";

import { AnimatedNumber, Badge, Button, Card, Checkbox, Empty, Group, Modal, Page, Segmented, Skeleton, cx, type Tone } from "../components/ui";
import { api } from "../lib/api";
import { ago, bytes, tildify } from "../lib/format";
import { useScanProgress, useStore } from "../lib/store";
import type { Category, CleanRequest, NodeModulesHit, RuleScan, Safety } from "../lib/types";

const CATEGORY: Record<Category, string> = {
  xcode: "Xcode & iOS Simulators",
  android: "Android & Gradle",
  java_script: "JavaScript",
  languages: "Other languages",
  tools: "Tools & editors",
  system: "System",
};
const SAFETY: Record<Safety, { label: string; tone: Tone }> = {
  safe: { label: "Safe", tone: "safe" },
  review: { label: "Review", tone: "warn" },
  report_only: { label: "Info only", tone: "neutral" },
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

  const toggleRule = (r: RuleScan, on: boolean) =>
    setSel((s) => ({ ...s, [r.rule.id]: new Set(on ? r.items.map((i) => i.path) : []) }));
  const toggleItem = (r: RuleScan, path: string, on: boolean) =>
    setSel((s) => {
      const next = new Set(s[r.rule.id]);
      if (on) next.add(path);
      else next.delete(path);
      return { ...s, [r.rule.id]: next };
    });

  const grouped = useMemo(() => {
    const g = new Map<Category, RuleScan[]>();
    for (const r of cleaner?.rules ?? []) g.set(r.rule.category, [...(g.get(r.rule.category) ?? []), r]);
    return [...g.entries()].sort((a, b) => sum(b[1]) - sum(a[1]));
  }, [cleaner]);

  return (
    <Page
      title="Dev Cleaner"
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
      {scanning && (
        <>
          <p className="mb-3 px-1 text-[12.5px] text-muted" aria-live="polite">
            Measuring caches… <span className="tabular font-medium text-ink">{bytes(progress?.bytes ?? 0)}</span> in{" "}
            <span className="tabular">{(progress?.files ?? 0).toLocaleString()}</span> files so far
          </p>
          <Skeleton rows={6} />
        </>
      )}

      {!cleaner && !scanning && (
        <Empty
          icon={<Sparkles className="size-7" aria-hidden />}
          title="Find developer junk"
          action={
            <Button variant="primary" onClick={scan}>
              Scan
            </Button>
          }
        >
          Scans Xcode DerivedData, iOS simulators, Android emulators, Gradle, npm/yarn/pnpm, CocoaPods, Homebrew and app caches. Nothing is deleted until you confirm.
        </Empty>
      )}

      {cleaner && !scanning && (
        <>
          {cleaner.xcode_installed === false && cleaner.rules.some((r) => r.rule.category === "xcode") && (
            <Card className="mb-4 flex items-start gap-3 border-warn/30 bg-warn-soft p-3.5 text-[13px]">
              <Info className="mt-0.5 size-4 shrink-0 text-warn-text" aria-hidden />
              <div>
                <b>Xcode isn't installed</b>, but its simulators and caches are still on disk. They're leftovers and safe to remove.
              </div>
            </Card>
          )}

          <Card className="sticky top-[60px] z-10 mb-6 flex items-center gap-4 bg-surface/90 p-3.5 pl-4 backdrop-blur-xl">
            <div className="flex-1">
              <div className="text-[17px] font-semibold tracking-[-0.01em]">
                <AnimatedNumber value={selected.bytes} format={bytes} /> selected
              </div>
              <div className="text-[11.5px] text-muted">
                {bytes(cleaner.total_bytes)} found · {bytes(cleaner.safe_bytes)} marked safe ·{" "}
                {deleteMode === "trash" ? "items go to the Trash" : "items are deleted permanently"}
              </div>
            </div>
            <Button variant="ghost" size="sm" onClick={() => setSel(defaultSelection(cleaner.rules))}>
              Select safe only
            </Button>
            <Button variant="primary" disabled={selected.count === 0} onClick={() => setConfirm(true)}>
              <Trash2 className="size-3.5" aria-hidden /> Clean
            </Button>
          </Card>

          {grouped.length === 0 && (
            <Empty icon={<Check className="size-7" aria-hidden />} title="Nothing to clean">
              No developer caches found. Nice and tidy.
            </Empty>
          )}

          <div>
            {grouped.map(([cat, rules]) => (
              <Group key={cat} title={CATEGORY[cat]} aside={bytes(sum(rules))}>
                <Card className="divide-y divide-line overflow-hidden">
                  {rules.map((r) => (
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
              </Group>
            ))}
          </div>
        </>
      )}

      <NodeModulesSection />

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

function sum(rules: RuleScan[]) {
  return rules.reduce((a, r) => a + r.total_bytes, 0);
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
        className="flex cursor-pointer items-center gap-3 px-4 py-2.5 transition-colors hover:bg-surface-2"
        onClick={onToggleOpen}
        role="button"
        tabIndex={0}
        aria-expanded={open}
        onKeyDown={(e) => (e.key === "Enter" || e.key === " ") && (e.preventDefault(), onToggleOpen())}
      >
        <Checkbox label={r.rule.name} checked={all} indeterminate={selected.size > 0} disabled={reportOnly} onChange={onRule} />
        <ChevronRight className={cx("size-3.5 shrink-0 text-faint transition-transform duration-200", open && "rotate-90")} aria-hidden />
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <span className="text-[13px] font-medium">{r.rule.name}</span>
            <Badge tone={SAFETY[r.rule.safety].tone}>{SAFETY[r.rule.safety].label}</Badge>
          </div>
          {r.note && r.rule.category !== "xcode" && <div className="mt-0.5 text-[11.5px] text-warn-text">{r.note}</div>}
        </div>
        <div className="tabular text-right text-[13px] font-semibold">{bytes(r.total_bytes)}</div>
      </div>
      {open && (
        <div className="border-t border-line bg-bg/60 px-4 pb-3 pl-[58px] pt-2.5">
          <p className="mb-2 text-[12px] text-muted">{r.rule.description}</p>
          {r.rule.command && (
            <div className="mb-2 flex items-center gap-2">
              <code className="selectable flex-1 truncate rounded-[6px] border border-line bg-surface px-2 py-1 font-mono text-[11.5px]">{r.rule.command}</code>
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

function NodeModulesSection() {
  const { home, deleteMode, reportDelete, toast } = useStore();
  const [hits, setHits] = useState<NodeModulesHit[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [days, setDays] = useState(90);
  const [sel, setSel] = useState<Set<string>>(new Set());
  const [removing, setRemoving] = useState(false);

  const find = async () => {
    setBusy(true);
    try {
      setHits(await api.nodeModulesFind(days));
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
      setHits((h) => h?.filter((x) => !gone.has(x.path)) ?? null);
      setSel(new Set());
    } finally {
      setRemoving(false);
    }
  };
  const selBytes = (hits ?? []).filter((h) => sel.has(h.path)).reduce((a, h) => a + h.bytes, 0);

  return (
    <section className="mt-10">
      <div className="mb-2 flex items-end justify-between gap-4 px-1">
        <div>
          <h2 className="text-[13px] font-semibold">Old node_modules</h2>
          <p className="text-[12px] text-muted">Projects you haven't touched in a while. `npm install` brings them back when you need them.</p>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <Segmented
            label="Untouched for"
            value={days}
            onChange={setDays}
            options={[
              { value: 30, label: "30 days" },
              { value: 90, label: "90 days" },
              { value: 180, label: "6 months" },
            ]}
          />
          <Button busy={busy} onClick={find}>
            <Package className="size-3.5" aria-hidden /> {hits ? "Search again" : "Find"}
          </Button>
        </div>
      </div>
      {hits && (
        <Card className="overflow-hidden">
          {hits.length === 0 ? (
            <div className="p-6 text-center text-[13px] text-muted">No node_modules folders found in your projects.</div>
          ) : (
            <>
              <div className="flex items-center gap-3 border-b border-line px-4 py-2.5">
                <span className="flex-1 text-[12.5px] text-muted">
                  {hits.length} projects · {bytes(hits.reduce((a, h) => a + h.bytes, 0))}
                </span>
                <Button size="sm" variant="ghost" onClick={() => setSel(new Set(hits.filter((h) => h.stale).map((h) => h.path)))}>
                  Select old ones
                </Button>
                <Button size="sm" variant="danger" disabled={sel.size === 0} busy={removing} onClick={remove}>
                  Remove {sel.size > 0 ? bytes(selBytes) : ""}
                </Button>
              </div>
              <ul className="divide-y divide-line">
                {hits.map((h) => (
                  <li key={h.path} className="flex items-center gap-3 px-4 py-2 text-[13px]">
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
                    <span className="selectable min-w-0 flex-1 truncate" title={h.path}>
                      {tildify(h.project, home)}
                    </span>
                    {h.stale && <Badge tone="warn">old</Badge>}
                    <span className="w-28 text-right text-[11.5px] text-faint">{ago(h.last_touched)}</span>
                    <span className="tabular w-16 text-right font-medium">{bytes(h.bytes)}</span>
                  </li>
                ))}
              </ul>
            </>
          )}
        </Card>
      )}
    </section>
  );
}
