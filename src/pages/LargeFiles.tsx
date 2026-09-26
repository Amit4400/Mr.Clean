import { ChevronRight, ExternalLink, File, Folder, FolderSearch, Home, StopCircle, Trash2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import { Badge, Button, Card, Checkbox, Empty, Modal, Page, Segmented, Skeleton, cx } from "../components/ui";
import { api } from "../lib/api";
import { ago, bytes, percent, tildify } from "../lib/format";
import { isProtected } from "../lib/paths";
import { useScanProgress, useStore } from "../lib/store";
import type { FileEntry, Kind, Node } from "../lib/types";

const KIND: Record<Kind, { label: string; color: string }> = {
  video: { label: "Videos", color: "#8b5cf6" },
  disk_image: { label: "Disk images", color: "#f59e0b" },
  installer: { label: "Installers", color: "#ef4444" },
  archive: { label: "Archives", color: "#3b82f6" },
  image: { label: "Images", color: "#ec4899" },
  audio: { label: "Audio", color: "#10b981" },
  document: { label: "Documents", color: "#06b6d4" },
  other: { label: "Everything else", color: "var(--c-faint)" },
};

const AGE_FILTERS = [
  { label: "Any age", days: 0 },
  { label: "Not changed in 3 months", days: 90 },
  { label: "Not changed in 6 months", days: 180 },
  { label: "Not changed in a year", days: 365 },
];

export default function LargeFiles() {
  const { files, setFiles, home, deleteMode, reportDelete, toast } = useStore();
  const [scanning, setScanning] = useState(false);
  const [tab, setTab] = useState<"folders" | "files">("folders");
  const [cwd, setCwd] = useState<string | null>(files?.root ?? null);
  const [nodes, setNodes] = useState<Node[]>([]);
  const [sel, setSel] = useState<Map<string, number>>(new Map());
  const [kind, setKind] = useState<Kind | "all">("all");
  const [age, setAge] = useState(0);
  const [confirm, setConfirm] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const progress = useScanProgress("bigfiles", scanning);

  useEffect(() => {
    if (!files) api.bigfilesSummary().then((s) => s && (setFiles(s), setCwd(s.root))).catch(() => {});
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    if (cwd && files) api.bigfilesChildren(cwd).then(setNodes).catch(() => setNodes([]));
  }, [cwd, files]);

  const scan = async () => {
    setScanning(true);
    setSel(new Map());
    try {
      const s = await api.bigfilesScan();
      setFiles(s);
      setCwd(s.root);
    } catch (e) {
      toast(String(e), "error");
    } finally {
      setScanning(false);
    }
  };

  const toggle = (path: string, b: number, on: boolean) =>
    setSel((s) => {
      const n = new Map(s);
      if (on) n.set(path, b);
      else n.delete(path);
      return n;
    });
  const selBytes = [...sel.values()].reduce((a, b) => a + b, 0);

  const remove = async () => {
    setDeleting(true);
    try {
      const r = await api.bigfilesRemove([...sel.keys()], deleteMode);
      reportDelete(r, deleteMode);
      setConfirm(false);
      setSel(new Map());
      const s = await api.bigfilesSummary();
      if (s) setFiles(s);
    } catch (e) {
      toast(String(e), "error");
    } finally {
      setDeleting(false);
    }
  };

  const crumbs = useMemo(() => {
    if (!files || !cwd) return [];
    const rel = cwd.slice(files.root.length).split("/").filter(Boolean);
    return [{ name: tildify(files.root, home), path: files.root }, ...rel.map((n, i) => ({ name: n, path: files.root + "/" + rel.slice(0, i + 1).join("/") }))];
  }, [files, cwd, home]);

  const topFiles = useMemo(() => {
    const cutoff = Date.now() / 1000 - age * 86400;
    return (files?.top_files ?? []).filter((f) => (kind === "all" || f.kind === kind) && (age === 0 || (f.modified ?? 0) < cutoff));
  }, [files, kind, age]);

  const parentTotal = nodes.reduce((a, n) => a + n.bytes, 0);

  return (
    <Page
        title="Large Files"
        subtitle="See which folders and files take the most space in your home folder, then pick what to remove."
        actions={
          scanning ? (
            <Button onClick={() => api.cancelScan()}>
              <StopCircle className="size-3.5" aria-hidden /> Stop
            </Button>
          ) : (
            <Button variant={files ? "secondary" : "primary"} onClick={scan}>
              <FolderSearch className="size-3.5" aria-hidden /> {files ? "Rescan" : "Scan home folder"}
            </Button>
          )
        }
    >
      {scanning && (
        <>
        <p className="mb-3 px-1 text-[12.5px] text-muted" aria-live="polite">
          Scanning… <span className="tabular font-medium text-ink">{(progress?.files ?? 0).toLocaleString()}</span> files,{" "}
          <span className="tabular font-medium text-ink">{bytes(progress?.bytes ?? 0)}</span>
        </p>
        <Skeleton rows={7} />
        </>
      )}

      {!files && !scanning && (
        <Empty icon={<FolderSearch className="size-7" aria-hidden />} title="Find what's using your space">
          Walks your whole home folder (usually under a minute) and shows the biggest folders and files. Nothing is changed until you choose.
        </Empty>
      )}

      {files && !scanning && (
        <>
          <Card className="mb-4 p-4">
            <div className="mb-2 flex items-baseline justify-between text-[13px]">
              <span className="font-semibold">{bytes(files.total_bytes)} in {files.file_count.toLocaleString()} files</span>
              {files.unreadable > 0 && <span className="text-[11.5px] text-faint">{files.unreadable} folders couldn't be read — see Settings → Full Disk Access</span>}
            </div>
            <div className="flex h-3 overflow-hidden rounded-full bg-surface-2">
              {files.kinds.map((k) => (
                <div key={k.kind} title={`${KIND[k.kind].label}: ${bytes(k.bytes)}`} style={{ width: `${percent(k.bytes, files.total_bytes)}%`, background: KIND[k.kind].color }} />
              ))}
            </div>
            <div className="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-[11.5px] text-muted">
              {files.kinds.map((k) => (
                <span key={k.kind} className="flex items-center gap-1.5">
                  <span className="size-2 rounded-full" style={{ background: KIND[k.kind].color }} />
                  {KIND[k.kind].label} <span className="tabular text-faint">{bytes(k.bytes)}</span>
                </span>
              ))}
            </div>
          </Card>

          <div className="mb-3 flex items-center gap-2">
            <Segmented
              label="View"
              value={tab}
              onChange={setTab}
              options={[
                { value: "folders", label: "Browse folders" },
                { value: "files", label: "Biggest files" },
              ]}
            />
            <div className="flex-1" />
            {sel.size > 0 && (
              <>
                <span className="tabular text-[13px] text-muted">
                  {sel.size} selected · {bytes(selBytes)}
                </span>
                <Button variant="ghost" size="sm" onClick={() => setSel(new Map())}>
                  Clear
                </Button>
                <Button variant="danger" onClick={() => setConfirm(true)}>
                  <Trash2 className="size-3.5" aria-hidden /> Remove
                </Button>
              </>
            )}
          </div>

          {tab === "folders" ? (
            <Card>
              <div className="flex flex-wrap items-center gap-1 border-b border-line px-4 py-2.5 text-[13px]">
                {crumbs.map((c, i) => (
                  <span key={c.path} className="flex items-center gap-1">
                    {i > 0 && <ChevronRight className="size-3.5 text-faint" />}
                    <button onClick={() => setCwd(c.path)} className={cx("rounded px-1 hover:bg-surface-2", i === crumbs.length - 1 ? "font-semibold" : "text-muted")}>
                      {i === 0 ? (
                        <span className="flex items-center gap-1">
                          <Home className="size-3.5" />
                          {c.name}
                        </span>
                      ) : (
                        c.name
                      )}
                    </button>
                  </span>
                ))}
              </div>
              <ul className="divide-y divide-line">
                {nodes.length === 0 && <li className="p-6 text-center text-[13px] text-muted">Empty folder</li>}
                {nodes.slice(0, 300).map((n) => (
                  <li key={n.path} className={cx("group flex items-center gap-3 px-4 py-2 text-[13px]", n.is_dir && "cursor-pointer hover:bg-surface-2")} onClick={() => n.is_dir && setCwd(n.path)}>
                    <Checkbox label={n.name} checked={sel.has(n.path)} disabled={isProtected(n.path, n.is_dir, home)} onChange={(on) => toggle(n.path, n.bytes, on)} />
                    {n.is_dir ? <Folder className="size-4 shrink-0 fill-accent/20 text-accent" aria-hidden /> : <File className="size-4 shrink-0 text-faint" aria-hidden />}
                    <span className="w-64 truncate" title={n.path}>
                      {n.name}
                    </span>
                    <div className="h-1.5 flex-1 overflow-hidden rounded-full bg-surface-2">
                      <div className="h-full rounded-full bg-accent/80 transition-[width] duration-500" style={{ width: `${percent(n.bytes, parentTotal)}%` }} />
                    </div>
                    <RevealButton path={n.path} />
                    <span className="tabular w-20 text-right font-medium">{bytes(n.bytes)}</span>
                  </li>
                ))}
              </ul>
            </Card>
          ) : (
            <Card>
              <div className="flex flex-wrap items-center gap-2 border-b border-line px-4 py-2.5">
                <select className="h-6 cursor-pointer rounded-[6px] border border-line bg-surface px-1.5 text-[12px] shadow-[0_1px_1px_rgba(0,0,0,0.05)]" aria-label="File type" value={kind} onChange={(e) => setKind(e.target.value as Kind | "all")}>
                  <option value="all">All types</option>
                  {Object.entries(KIND).map(([k, v]) => (
                    <option key={k} value={k}>
                      {v.label}
                    </option>
                  ))}
                </select>
                <select className="h-6 cursor-pointer rounded-[6px] border border-line bg-surface px-1.5 text-[12px] shadow-[0_1px_1px_rgba(0,0,0,0.05)]" aria-label="Last changed" value={age} onChange={(e) => setAge(Number(e.target.value))}>
                  {AGE_FILTERS.map((a) => (
                    <option key={a.days} value={a.days}>
                      {a.label}
                    </option>
                  ))}
                </select>
                <span className="text-[11.5px] text-faint">Files over 10 MB</span>
              </div>
              <ul className="divide-y divide-line">
                {topFiles.length === 0 && <li className="p-6 text-center text-[13px] text-muted">No files match these filters.</li>}
                {topFiles.map((f) => (
                  <FileRow key={f.path} f={f} home={home} checked={sel.has(f.path)} onToggle={(on) => toggle(f.path, f.bytes, on)} />
                ))}
              </ul>
            </Card>
          )}
        </>
      )}

      <Modal
        open={confirm}
        title={`Remove ${sel.size} item${sel.size === 1 ? "" : "s"} (${bytes(selBytes)})?`}
        onClose={() => !deleting && setConfirm(false)}
        footer={
          <>
            <Button onClick={() => setConfirm(false)} disabled={deleting}>
              Cancel
            </Button>
            <Button variant="danger" busy={deleting} onClick={remove}>
              {deleteMode === "trash" ? "Move to Trash" : "Delete permanently"}
            </Button>
          </>
        }
      >
        <p className="mb-3 text-muted">These are your own files — double-check before removing them.</p>
        <ul className="space-y-1 text-[11.5px]">
          {[...sel.entries()].map(([p, b]) => (
            <li key={p} className="flex justify-between gap-4">
              <span className="selectable truncate">{tildify(p, home)}</span>
              <span className="tabular shrink-0 font-medium">{bytes(b)}</span>
            </li>
          ))}
        </ul>
        <p className="mt-3 text-[11.5px] text-faint">Protected locations (app data in ~/Library, .ssh, .git folders, system files) are refused automatically.</p>
      </Modal>
    </Page>
  );
}

function RevealButton({ path }: { path: string }) {
  return (
    <button
      title="Show in Finder"
      className="rounded p-1 text-faint opacity-0 transition hover:bg-surface hover:text-ink group-hover:opacity-100"
      onClick={(e) => {
        e.stopPropagation();
        api.reveal(path);
      }}
    >
      <ExternalLink className="size-3.5" />
    </button>
  );
}

function FileRow({ f, home, checked, onToggle }: { f: FileEntry; home: string | null; checked: boolean; onToggle: (on: boolean) => void }) {
  return (
    <li className="group flex items-center gap-3 px-4 py-2 text-[13px]">
      <Checkbox label={f.name} checked={checked} disabled={isProtected(f.path, false, home)} onChange={onToggle} />
      <div className="min-w-0 flex-1">
        <div className="truncate font-medium">{f.name}</div>
        <div className="selectable truncate text-[11.5px] text-faint" title={f.path}>
          {tildify(f.path, home)}
        </div>
      </div>
      <Badge>{KIND[f.kind].label}</Badge>
      <span className="w-28 text-right text-[11.5px] text-faint">{ago(f.modified)}</span>
      <RevealButton path={f.path} />
      <span className="tabular w-20 text-right font-semibold">{bytes(f.bytes)}</span>
    </li>
  );
}
