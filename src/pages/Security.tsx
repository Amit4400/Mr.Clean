import { Archive, ExternalLink, Info, RotateCcw, ShieldAlert, ShieldCheck, StopCircle } from "lucide-react";
import { motion } from "motion/react";
import { useEffect, useMemo, useState } from "react";

import { Badge, Button, Card, Empty, Page, Skeleton, cx, type Tone } from "../components/ui";
import { api } from "../lib/api";
import { ago, tildify } from "../lib/format";
import { useStore } from "../lib/store";
import type { Area, Finding, QuarantineEntry, Severity } from "../lib/types";

const SEV: Record<Severity, { label: string; tone: Tone; bar: string }> = {
  high: { label: "High", tone: "danger", bar: "bg-danger" },
  medium: { label: "Medium", tone: "warn", bar: "bg-warn" },
  low: { label: "Low", tone: "info", bar: "bg-info" },
  info: { label: "Info", tone: "neutral", bar: "bg-faint" },
};
const AREA: Record<Area, string> = {
  known_malware: "Known malware",
  npm: "npm packages",
  git: "Git",
  startup: "Startup items",
  shell_profile: "Shell profile",
  secrets: "Exposed secrets",
  project_code: "Project code",
};

export default function Security() {
  const { security, setSecurity, home, toast } = useStore();
  const [scanning, setScanning] = useState(false);
  const [show, setShow] = useState<Set<Severity>>(new Set(["high", "medium", "low"]));
  const [quarantine, setQuarantine] = useState<QuarantineEntry[]>([]);
  const [done, setDone] = useState<Set<string>>(new Set());

  const loadQuarantine = () => api.quarantineList().then(setQuarantine).catch(() => {});
  useEffect(() => {
    loadQuarantine();
  }, []);

  const scan = async () => {
    setScanning(true);
    try {
      setSecurity(await api.securityScan());
      setDone(new Set());
    } catch (e) {
      toast(String(e), "error");
    } finally {
      setScanning(false);
    }
  };

  const doQuarantine = async (f: Finding) => {
    try {
      await api.securityQuarantine(f.id);
      setDone((d) => new Set(d).add(f.id));
      toast("Moved to quarantine. You can restore it below.");
      loadQuarantine();
    } catch (e) {
      toast(String(e), "error");
    }
  };

  const restore = async (q: QuarantineEntry) => {
    try {
      await api.quarantineRestore(q.id);
      toast("Restored to its original location.");
      loadQuarantine();
    } catch (e) {
      toast(String(e), "error");
    }
  };

  const visible = useMemo(() => (security?.findings ?? []).filter((f) => show.has(f.severity)), [security, show]);
  const serious = security ? security.counts.high + security.counts.medium : 0;

  return (
    <Page
        title="Security"
        subtitle="Looks for malware that targets developers: npm worms, fake-interview projects, startup items, git hooks and exposed tokens."
        actions={
          scanning ? (
            <Button onClick={() => api.cancelScan()}>
              <StopCircle className="size-3.5" aria-hidden /> Stop
            </Button>
          ) : (
            security && (
              <Button onClick={scan}>
                <ShieldCheck className="size-3.5" aria-hidden /> Scan again
              </Button>
            )
          )
        }
    >

      {scanning && (
        <>
          <p className="mb-3 px-1 text-[12.5px] text-muted" aria-live="polite">
            Checking startup items, shell profiles, git repositories and npm packages…
          </p>
          <Skeleton rows={5} />
        </>
      )}

      {!security && !scanning && (
        <Empty
          icon={<ShieldCheck className="size-7" aria-hidden />}
          title="Check this Mac for developer-targeted malware"
          action={
            <Button variant="primary" onClick={scan}>
              Scan
            </Button>
          }
        >
          Read-only scan. Nothing is changed unless you choose to quarantine an item, and quarantined items can be restored.
        </Empty>
      )}

      {security && !scanning && (
        <>
          <Card className={cx("mb-4 flex items-center gap-4 p-5", serious > 0 ? "border-danger/40" : "border-accent/40")}>
            <div className={cx("rounded-2xl p-3", serious > 0 ? "bg-danger-soft text-danger" : "bg-accent-soft text-accent")}>
              {serious > 0 ? <ShieldAlert className="size-6" aria-hidden /> : <ShieldCheck className="size-6" aria-hidden />}
            </div>
            <div className="flex-1">
              <div className="text-lg font-semibold">{serious > 0 ? `${serious} item${serious > 1 ? "s" : ""} need your attention` : "No threats found"}</div>
              <div className="text-[11.5px] text-muted">
                {security.scanned_repos} git repos, {security.scanned_projects} projects and {security.scanned_packages.toLocaleString()} npm packages checked in{" "}
                {(security.duration_ms / 1000).toFixed(1)} s
              </div>
            </div>
            <div className="flex gap-1.5">
              {(Object.keys(SEV) as Severity[]).reverse().map((s) => (
                <button
                  key={s}
                  onClick={() =>
                    setShow((cur) => {
                      const n = new Set(cur);
                      if (n.has(s)) n.delete(s);
                      else n.add(s);
                      return n;
                    })
                  }
                  className={cx("rounded-lg border px-2.5 py-1.5 text-center transition", show.has(s) ? "border-line bg-surface-2" : "border-transparent opacity-50")}
                >
                  <div className="tabular text-[13px] font-semibold">{security.counts[s]}</div>
                  <div className="text-[11px] text-muted">{SEV[s].label}</div>
                </button>
              ))}
            </div>
          </Card>

          {serious > 0 && (
            <Card className="mb-4 flex gap-3 bg-info-soft p-3.5 text-[13px]">
              <Info className="mt-0.5 size-4 shrink-0 text-info-text" aria-hidden />
              <div className="text-muted">
                <b className="text-ink">If something here is real:</b> disconnect from sensitive work, quarantine the item, then rotate your GitHub, npm and cloud tokens
                from another device. On GitHub, check <i>Settings → Security log</i> and your repositories for commits or repos you didn't make.
              </div>
            </Card>
          )}

          <div className="space-y-2.5">
            {visible.length === 0 && <div className="py-8 text-center text-[13px] text-muted">Nothing to show with these filters.</div>}
            {visible.map((f, i) => (
              <motion.div key={f.id} initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: Math.min(i, 10) * 0.03, duration: 0.25 }}>
                <FindingCard f={f} home={home} quarantined={done.has(f.id)} onQuarantine={() => doQuarantine(f)} />
              </motion.div>
            ))}
          </div>
        </>
      )}

      {quarantine.length > 0 && (
        <section className="mt-8">
          <h2 className="mb-2 flex items-center gap-2 px-1 text-[13px] font-semibold">
            <Archive className="size-4 text-muted" /> Quarantine
          </h2>
          <Card className="divide-y divide-line">
            {quarantine.map((q) => (
              <div key={q.id} className="flex items-center gap-3 px-4 py-2.5 text-[13px]">
                <div className="min-w-0 flex-1">
                  <div className="selectable truncate font-medium">{tildify(q.original, home)}</div>
                  <div className="text-[11.5px] text-faint">
                    {q.reason} · {ago(q.at)}
                  </div>
                </div>
                <Button size="sm" onClick={() => restore(q)}>
                  <RotateCcw className="size-3.5" /> Restore
                </Button>
              </div>
            ))}
          </Card>
          <p className="mt-2 px-1 text-[11.5px] text-faint">Stored in ~/.mrclean/quarantine with execute permission removed.</p>
        </section>
      )}

      <p className="mt-8 text-[11.5px] text-faint">
        Mr.Clean spots known developer-targeted threats and suspicious patterns. It complements, but doesn't replace, a full antivirus such as XProtect or your company's endpoint protection.
      </p>
    </Page>
  );
}

function FindingCard({ f, home, quarantined, onQuarantine }: { f: Finding; home: string | null; quarantined: boolean; onQuarantine: () => void }) {
  const s = SEV[f.severity];
  return (
    <Card className={cx("relative overflow-hidden", quarantined && "opacity-50")}>
      <div className={cx("absolute inset-y-0 left-0 w-1", s.bar)} />
      <div className="py-3.5 pl-5 pr-4">
        <div className="flex items-start gap-3">
          <div className="min-w-0 flex-1">
            <div className="flex flex-wrap items-center gap-2">
              <Badge tone={s.tone}>{s.label}</Badge>
              <span className="text-[11.5px] text-faint">{AREA[f.area]}</span>
            </div>
            <div className="mt-1 font-semibold">{f.title}</div>
            <p className="mt-0.5 text-[13px] text-muted">{f.detail}</p>
          </div>
          <div className="flex shrink-0 gap-1.5">
            {f.path && (
              <Button size="sm" variant="ghost" onClick={() => api.reveal(f.path!)} title="Show in Finder">
                <ExternalLink className="size-3.5" />
              </Button>
            )}
            {f.can_quarantine && (
              <Button size="sm" variant={f.severity === "high" ? "danger" : "secondary"} disabled={quarantined} onClick={onQuarantine}>
                <Archive className="size-3.5" /> {quarantined ? "Quarantined" : "Quarantine"}
              </Button>
            )}
          </div>
        </div>
        {f.path && (
          <div className="selectable mt-2 truncate font-mono text-[11.5px] text-faint" title={f.path}>
            {tildify(f.path, home)}
            {f.line ? `:${f.line}` : ""}
          </div>
        )}
        {f.evidence && <pre className="selectable mt-2 overflow-x-auto whitespace-pre-wrap break-all rounded-md bg-surface-2 px-2.5 py-1.5 font-mono text-[11.5px]">{f.evidence}</pre>}
        {f.advice && <p className="mt-2 text-[11.5px] text-muted">→ {f.advice}</p>}
      </div>
    </Card>
  );
}
