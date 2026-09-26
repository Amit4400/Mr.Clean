import { Archive, Bug, CircleAlert, CircleCheck, CircleHelp, Wrench, ExternalLink, FileCode2, GitBranch, Info, KeyRound, Package, Power, RotateCcw, ShieldAlert, ShieldCheck, StopCircle, SquareTerminal, type LucideIcon } from "lucide-react";
import { motion } from "motion/react";
import { useEffect, useMemo, useState } from "react";

import { Badge, Button, Card, Empty, Modal, Page, Skeleton, SoftTile, cx, type Tone } from "../components/ui";
import { api } from "../lib/api";
import { ago, tildify } from "../lib/format";
import { useStore } from "../lib/store";
import type { Area, Finding, Fix, ProtectionCheck, QuarantineEntry, SecurityReport, Severity } from "../lib/types";

const SEV: Record<Severity, { label: string; tone: Tone; color: string }> = {
  high: { label: "High", tone: "danger", color: "var(--c-danger)" },
  medium: { label: "Medium", tone: "warn", color: "var(--c-warn)" },
  low: { label: "Low", tone: "info", color: "var(--c-info)" },
  info: { label: "Info", tone: "neutral", color: "var(--c-faint)" },
};

// Every area the scanner checks, in the order shown under "What we checked".
const AREA: Record<Area, { label: string; checks: string; icon: LucideIcon; color: string }> = {
  known_malware: { label: "Known malware", checks: "Shai-Hulud, BeaverTail and other known folders", icon: Bug, color: "var(--c-danger)" },
  npm: { label: "npm packages", checks: "Known bad versions and risky install scripts", icon: Package, color: "var(--c-danger)" },
  project_code: { label: "Project code", checks: "Hidden code in config files", icon: FileCode2, color: "var(--c-warn)" },
  startup: { label: "Startup items", checks: "Launch agents and login items", icon: Power, color: "var(--c-purple)" },
  shell_profile: { label: "Shell profiles", checks: ".zshrc, .bashrc and friends", icon: SquareTerminal, color: "var(--c-teal)" },
  git: { label: "Git", checks: "Hooks, identity and commits", icon: GitBranch, color: "var(--c-info)" },
  secrets: { label: "Exposed tokens", checks: "Tokens in dotfiles and shell history", icon: KeyRound, color: "var(--c-warn)" },
};

export default function Security() {
  const { security, setSecurity, home, toast } = useStore();
  const [scanning, setScanning] = useState(false);
  const [show, setShow] = useState<Set<Severity>>(new Set(["high", "medium", "low"]));
  const [quarantine, setQuarantine] = useState<QuarantineEntry[]>([]);
  const [done, setDone] = useState<Set<string>>(new Set());

  const [checks, setChecks] = useState<ProtectionCheck[] | null>(null);
  const [fixing, setFixing] = useState<Finding | null>(null);
  const [fixBusy, setFixBusy] = useState(false);

  const loadQuarantine = () => api.quarantineList().then(setQuarantine).catch(() => {});
  useEffect(() => {
    loadQuarantine();
    // Protection checks are quick and read-only, so they show without a scan.
    api.protectionChecks().then(setChecks).catch(() => setChecks([]));
  }, []);

  const applyFix = async () => {
    if (!fixing) return;
    setFixBusy(true);
    try {
      toast(await api.securityFix(fixing.id));
      setFixing(null);
      await scan();
    } catch (e) {
      toast(String(e), "error");
    } finally {
      setFixBusy(false);
    }
  };

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
      const q = await api.securityQuarantine(f.id);
      setDone((d) => new Set(d).add(f.id));
      toast(q.stopped ? "Moved to quarantine and stopped it. You can restore it below." : "Moved to quarantine. You can restore it below.");
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
        <Card className="p-5">
          <p className="mb-3 text-[13px] text-muted" aria-live="polite">
            Checking startup items, shell profiles, git repositories and npm packages…
          </p>
          <Skeleton rows={5} />
        </Card>
      )}

      {!security && !scanning && checks && checks.length > 0 && <Protection checks={checks} />}

      {!security && !scanning && (
        <Card>
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
        </Card>
      )}

      {security && !scanning && (
        <>
          {/* Summary with severity counts that double as filters. */}
          <Card className="relative mb-4 overflow-hidden p-5">
            <div
              className="pointer-events-none absolute -left-12 -top-16 size-48 rounded-full"
              style={{ background: `radial-gradient(circle, color-mix(in srgb, ${serious ? "var(--c-danger)" : "var(--c-safe)"} 22%, transparent), transparent 65%)` }}
              aria-hidden
            />
            <div className="relative flex items-center gap-4">
              <SoftTile icon={serious > 0 ? ShieldAlert : ShieldCheck} color={serious > 0 ? "var(--c-danger)" : "var(--c-safe)"} size={56} round />
              <div className="min-w-0 flex-1">
                <div className="text-[20px] font-bold tracking-[-0.01em]">{serious > 0 ? `${serious} item${serious > 1 ? "s" : ""} need your attention` : "No threats found"}</div>
                <div className="text-[12.5px] text-muted">
                  {security.scanned_repos} git repos, {security.scanned_projects} projects and {security.scanned_packages.toLocaleString()} npm packages checked in{" "}
                  {(security.duration_ms / 1000).toFixed(1)} s
                </div>
              </div>
              <div className="flex gap-2" role="group" aria-label="Show severities">
                {(Object.keys(SEV) as Severity[]).reverse().map((s) => (
                  <button
                    key={s}
                    aria-pressed={show.has(s)}
                    onClick={() =>
                      setShow((cur) => {
                        const n = new Set(cur);
                        if (n.has(s)) n.delete(s);
                        else n.add(s);
                        return n;
                      })
                    }
                    className={cx("w-[68px] cursor-pointer rounded-[12px] border px-2 py-2 text-center transition", show.has(s) ? "border-transparent" : "border-line opacity-45")}
                    style={show.has(s) ? { background: `color-mix(in srgb, ${SEV[s].color} 14%, transparent)` } : undefined}
                  >
                    <div className="tabular text-[18px] font-bold leading-tight" style={{ color: security.counts[s] ? SEV[s].color : undefined }}>
                      {security.counts[s]}
                    </div>
                    <div className="text-[11px] text-muted">{SEV[s].label}</div>
                  </button>
                ))}
              </div>
            </div>
            {serious > 0 && (
              <div className="relative mt-4 flex gap-2 rounded-[12px] bg-info-soft px-3 py-2 text-[12.5px]">
                <Info className="mt-0.5 size-3.5 shrink-0 text-info-text" aria-hidden />
                <div className="text-muted">
                  <b className="text-ink">If something here is real:</b> disconnect from sensitive work, quarantine the item, then rotate your GitHub, npm and cloud tokens from
                  another device. On GitHub, check <i>Settings → Security log</i> for anything you didn't do.
                </div>
              </div>
            )}
          </Card>

          {checks && checks.length > 0 && <Protection checks={checks} />}

          <div className="grid grid-cols-[minmax(0,1fr)_280px] items-start gap-5">
            <Card className="divide-y divide-line overflow-hidden">
              {visible.length === 0 && (
                <div className="flex flex-col items-center gap-2 py-10 text-center text-[13px] text-muted">
                  <CircleCheck className="size-6 text-safe" aria-hidden />
                  {security.findings.length ? "Nothing to show with these filters." : "Nothing suspicious found. See what we checked on the right."}
                </div>
              )}
              {visible.map((f, i) => (
                <motion.div key={f.id} initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: Math.min(i, 10) * 0.04, duration: 0.3 }}>
                  <FindingRow f={f} home={home} quarantined={done.has(f.id)} onQuarantine={() => doQuarantine(f)} onFix={() => setFixing(f)} />
                </motion.div>
              ))}
            </Card>

            <Checked report={security} />
          </div>
        </>
      )}

      {quarantine.length > 0 && (
        <section className="mt-6">
          <h2 className="mb-2 flex items-center gap-2 px-1 text-[15px] font-semibold">
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

      <p className="mt-6 text-[11.5px] text-faint">
        Mr.Clean spots known developer-targeted threats and suspicious patterns. It complements, but doesn't replace, a full antivirus such as XProtect or your company's endpoint
        protection.
      </p>
      <Modal
        open={!!fixing}
        title="Apply this fix?"
        onClose={() => !fixBusy && setFixing(null)}
        footer={
          <>
            <Button onClick={() => setFixing(null)} disabled={fixBusy}>
              Cancel
            </Button>
            <Button variant="primary" busy={fixBusy} onClick={applyFix}>
              Apply fix
            </Button>
          </>
        }
      >
        {fixing?.fix && (
          <>
            <p className="mb-2">{FIX[fixing.fix].what}</p>
            <code className="selectable block rounded-[8px] bg-ink/[0.05] px-2.5 py-1.5 font-mono text-[11.5px]">{FIX[fixing.fix].command}</code>
            <p className="mt-2 text-[12px] text-muted">This changes your global git settings. You can undo it by running the opposite command.</p>
          </>
        )}
      </Modal>
    </Page>
  );
}

const FIX: Record<Fix, { what: string; command: string }> = {
  unset_global_hooks_path: { what: "Stop every repository from running the global hooks folder.", command: "git config --global --unset core.hooksPath" },
  use_keychain_credentials: { what: "Store git passwords in the macOS Keychain instead of a plain text file.", command: "git config --global credential.helper osxkeychain" },
};

const CHECK_STATE = {
  pass: { icon: CircleCheck, color: "var(--c-safe)", label: "On" },
  fail: { icon: CircleAlert, color: "var(--c-danger)", label: "Off" },
  unknown: { icon: CircleHelp, color: "var(--c-warn)", label: "Couldn't check" },
} as const;

/** FileVault, Firewall and friends: read-only checks with a way to fix each. */
function Protection({ checks }: { checks: ProtectionCheck[] }) {
  const off = checks.filter((c) => c.state !== "pass").length;
  return (
    <Card className="mb-4 p-4">
      <div className="mb-3 flex items-baseline justify-between px-1">
        <h2 className="text-[15px] font-semibold">Mac protection</h2>
        <span className={cx("text-[12px] font-medium", off ? "text-warn-text" : "text-safe-text")}>
          {off ? `${off} to review` : "All protections on"}
        </span>
      </div>
      <ul className="grid grid-cols-3 gap-2">
        {checks.map((c) => {
          const st = CHECK_STATE[c.state];
          return (
            <li key={c.id} className="flex items-center gap-2.5 rounded-[12px] bg-ink/[0.025] px-3 py-2.5" title={c.how ?? c.about}>
              <st.icon className="size-5 shrink-0" style={{ color: st.color }} aria-label={st.label} />
              <div className="min-w-0 flex-1">
                <div className="truncate text-[12.5px] font-semibold">{c.title}</div>
                <div className="truncate text-[11px] text-muted">{c.state === "pass" ? c.about : c.how ?? c.about}</div>
              </div>
              {c.state !== "pass" && c.pane && (
                <Button size="sm" variant={c.state === "fail" ? "primary" : "secondary"} onClick={() => api.openSettings(c.pane!)}>
                  {c.state === "fail" ? "Fix" : "Check"}
                </Button>
              )}
            </li>
          );
        })}
      </ul>
      <p className="mt-2 px-1 text-[11px] text-faint">Fix opens the right page in System Settings. Mr.Clean never changes these for you.</p>
    </Card>
  );
}

/** Every area the scan covered, with a tick or the number of issues found. */
function Checked({ report }: { report: SecurityReport }) {
  const issues = (a: Area) => report.findings.filter((f) => f.area === a && f.severity !== "info").length;
  return (
    <Card className="sticky top-4 p-4">
      <h2 className="mb-2 text-[15px] font-semibold">What we checked</h2>
      <ul className="space-y-0.5">
        {(Object.keys(AREA) as Area[]).map((a) => {
          const n = issues(a);
          const A = AREA[a];
          return (
            <li key={a} className="flex items-center gap-2.5 py-1.5">
              <SoftTile icon={A.icon} color={A.color} size={30} />
              <div className="min-w-0 flex-1">
                <div className="text-[12.5px] font-medium">{A.label}</div>
                <div className="truncate text-[11px] text-faint">{A.checks}</div>
              </div>
              {n > 0 ? (
                <span className="text-[11.5px] font-semibold text-danger-text">
                  {n} issue{n > 1 ? "s" : ""}
                </span>
              ) : (
                <CircleCheck className="size-4 shrink-0 text-safe" aria-label="No issues" />
              )}
            </li>
          );
        })}
      </ul>
    </Card>
  );
}

function FindingRow({ f, home, quarantined, onQuarantine, onFix }: { f: Finding; home: string | null; quarantined: boolean; onQuarantine: () => void; onFix: () => void }) {
  const [open, setOpen] = useState(false);
  const A = AREA[f.area];
  return (
    <div className={cx(quarantined && "opacity-50")}>
      <div className="flex items-start gap-3 px-4 py-3.5">
        <SoftTile icon={A.icon} color={SEV[f.severity].color} size={40} />
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <span className="text-[13.5px] font-semibold">{f.title}</span>
            <Badge tone={SEV[f.severity].tone}>{SEV[f.severity].label}</Badge>
          </div>
          <p className="mt-0.5 text-[12.5px] text-muted">{f.detail}</p>
          {f.path && (
            <div className="selectable mt-1 truncate font-mono text-[11px] text-faint" title={f.path}>
              {tildify(f.path, home)}
              {f.line ? `:${f.line}` : ""}
            </div>
          )}
          <div className="mt-2 flex flex-wrap items-center gap-1.5">
            {(f.evidence || f.advice) && (
              <Button size="sm" onClick={() => setOpen((o) => !o)} aria-expanded={open}>
                {open ? "Hide details" : "View details"}
              </Button>
            )}
            {f.path && (
              <Button size="sm" onClick={() => api.reveal(f.path!)}>
                <ExternalLink className="size-3.5" aria-hidden /> Show in Finder
              </Button>
            )}
            {f.fix && (
              <Button size="sm" variant="primary" onClick={onFix}>
                <Wrench className="size-3.5" aria-hidden /> Fix
              </Button>
            )}
            {f.can_quarantine && (
              <Button size="sm" variant={f.severity === "high" ? "danger" : "secondary"} disabled={quarantined} onClick={onQuarantine}>
                <Archive className="size-3.5" /> {quarantined ? "Quarantined" : "Quarantine"}
              </Button>
            )}
          </div>
          {open && (f.evidence || f.advice) && (
            <div className="mt-2.5">
              {f.evidence && <pre className="selectable overflow-x-auto whitespace-pre-wrap break-all rounded-[10px] bg-ink/[0.05] px-3 py-2 font-mono text-[11.5px]">{f.evidence}</pre>}
              {f.advice && <p className="mt-2 text-[12px] text-muted">→ {f.advice}</p>}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
