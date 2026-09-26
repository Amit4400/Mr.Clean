import { CheckCircle2, HardDrive, ShieldAlert, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";

import { Button, Card, Page, cx } from "../components/ui";
import { api, inTauri, openFullDiskAccessSettings } from "../lib/api";
import { useStore } from "../lib/store";
import type { DeleteMode } from "../lib/types";

export default function Settings() {
  const { deleteMode, setDeleteMode } = useStore();
  const [fda, setFda] = useState<boolean | null>(null);

  useEffect(() => {
    api.hasFullDiskAccess().then(setFda).catch(() => setFda(null));
  }, []);

  const option = (mode: DeleteMode, title: string, text: string) => (
    <button
      onClick={() => setDeleteMode(mode)}
      className={cx("flex-1 rounded-xl border p-4 text-left transition", deleteMode === mode ? "border-accent bg-accent-soft" : "border-line hover:bg-surface-2")}
    >
      <div className="font-semibold">{title}</div>
      <div className="mt-1 text-[11.5px] text-muted">{text}</div>
    </button>
  );

  return (
    <Page title="Settings">

      <Card className="mb-4 p-5">
        <div className="mb-3 flex items-center gap-2 font-semibold">
          <Trash2 className="size-4 text-muted" /> When cleaning
        </div>
        <div className="flex gap-3">
          {option("trash", "Move to Trash (recommended)", "Undo-able: put things back from the Trash. Empty the Trash to actually free the space.")}
          {option("permanent", "Delete permanently", "Frees space immediately. Can't be undone.")}
        </div>
      </Card>

      <Card className="mb-4 p-5">
        <div className="mb-2 flex items-center gap-2 font-semibold">
          <HardDrive className="size-4 text-muted" /> Full Disk Access
        </div>
        {fda ? (
          <p className="flex items-center gap-2 text-[13px] text-accent">
            <CheckCircle2 className="size-4" /> Granted. Mr.Clean can see every cache folder.
          </p>
        ) : (
          <>
            <p className="text-[13px] text-muted">
              macOS hides some folders (Trash, Mail, parts of ~/Library) until you allow it. Without it, scans still work but may miss some space. Open the setting, turn on
              <b> Mr.Clean</b>, then restart the app.
            </p>
            <Button className="mt-3" onClick={() => openFullDiskAccessSettings()} disabled={!inTauri}>
              Open Privacy settings
            </Button>
          </>
        )}
      </Card>

      <Card className="p-5">
        <div className="mb-2 flex items-center gap-2 font-semibold">
          <ShieldAlert className="size-4 text-muted" /> Security scanner data
        </div>
        <ul className="list-disc space-y-1 pl-5 text-[13px] text-muted">
          <li>
            Quarantined items live in <code className="font-mono text-[11.5px]">~/.mrclean/quarantine</code> and can be restored from the Security page.
          </li>
          <li>
            To add newly published malicious package versions, place a JSON file at <code className="font-mono text-[11.5px]">~/.mrclean/iocs.json</code> using the same format as
            the bundled list. It's merged on the next scan.
          </li>
          <li>Nothing is ever uploaded. All scanning happens on this Mac.</li>
        </ul>
      </Card>

      <p className="mt-6 text-[11.5px] text-faint">Mr.Clean 0.1.0</p>
    </Page>
  );
}
