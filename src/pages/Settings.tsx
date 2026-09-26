import { CircleCheck, HardDrive, Lock, ShieldAlert, Trash2, type LucideIcon } from "lucide-react";
import { useEffect, useState, type ReactNode } from "react";

import { Button, Card, Page, SoftTile, cx } from "../components/ui";
import { api, inTauri, openFullDiskAccessSettings } from "../lib/api";
import { useStore } from "../lib/store";
import type { DeleteMode } from "../lib/types";

function Section({ icon, color, title, children }: { icon: LucideIcon; color: string; title: string; children: ReactNode }) {
  return (
    <Card className="flex gap-4 p-5">
      <SoftTile icon={icon} color={color} size={42} />
      <div className="min-w-0 flex-1">
        <h2 className="mb-2 text-[15px] font-semibold">{title}</h2>
        {children}
      </div>
    </Card>
  );
}

export default function Settings() {
  const { deleteMode, setDeleteMode } = useStore();
  const [fda, setFda] = useState<boolean | null>(null);

  useEffect(() => {
    api.hasFullDiskAccess().then(setFda).catch(() => setFda(null));
  }, []);

  const option = (mode: DeleteMode, title: string, text: string) => (
    <button
      onClick={() => setDeleteMode(mode)}
      aria-pressed={deleteMode === mode}
      className={cx(
        "flex flex-1 cursor-pointer items-start gap-3 rounded-[14px] border p-4 text-left transition",
        deleteMode === mode ? "border-accent/50 bg-accent-soft" : "glass hover:brightness-[1.02]",
      )}
    >
      <span className={cx("mt-0.5 flex size-4 shrink-0 items-center justify-center rounded-full border-2", deleteMode === mode ? "border-accent" : "border-faint/60")}>
        {deleteMode === mode && <span className="size-2 rounded-full bg-accent" />}
      </span>
      <span>
        <span className="block font-semibold">{title}</span>
        <span className="mt-1 block text-[12px] text-muted">{text}</span>
      </span>
    </button>
  );

  return (
    <Page title="Settings" subtitle="How Mr.Clean removes things, and what it can see.">
      <div className="flex flex-col gap-4">
        <Section icon={Trash2} color="var(--c-accent)" title="When cleaning">
          <div className="flex gap-3">
            {option("trash", "Move to Trash (recommended)", "Undo-able: put things back from the Trash. Empty the Trash to actually free the space.")}
            {option("permanent", "Delete permanently", "Frees space immediately. Can't be undone.")}
          </div>
        </Section>

        <Section icon={HardDrive} color="var(--c-info)" title="Full Disk Access">
          {fda ? (
            <p className="flex items-center gap-2 text-[13px] text-safe-text">
              <CircleCheck className="size-4" /> Granted. Mr.Clean can see every cache folder.
            </p>
          ) : (
            <>
              <p className="text-[13px] text-muted">
                macOS hides some folders (Trash, Mail, parts of ~/Library) until you allow it. Without it, scans still work but may miss some space. Open the setting, turn on
                <b className="text-ink"> Mr.Clean</b>, then restart the app.
              </p>
              <Button className="mt-3" onClick={() => openFullDiskAccessSettings()} disabled={!inTauri}>
                Open Privacy settings
              </Button>
            </>
          )}
        </Section>

        <Section icon={ShieldAlert} color="var(--c-danger)" title="Security scanner data">
          <ul className="list-disc space-y-1 pl-5 text-[13px] text-muted">
            <li>
              Quarantined items live in <code className="font-mono text-[11.5px]">~/.mrclean/quarantine</code> and can be restored from the Security page.
            </li>
            <li>
              To add newly published malicious package versions, place a JSON file at <code className="font-mono text-[11.5px]">~/.mrclean/iocs.json</code> using the same format as
              the bundled list. It's merged on the next scan.
            </li>
          </ul>
        </Section>

        <Section icon={Lock} color="var(--c-purple)" title="Privacy">
          <p className="text-[13px] text-muted">Nothing is ever uploaded. No accounts, no tracking, no automatic updates. All scanning happens on this Mac.</p>
        </Section>
      </div>

      <p className="mt-5 text-[11.5px] text-faint">Mr.Clean 0.1.0</p>
    </Page>
  );
}
