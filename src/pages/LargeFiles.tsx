import { ChevronRight, ExternalLink, File, FolderClosed, FolderOpen, FolderSearch, Home, Search, StopCircle, Trash2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import { Badge, Button, Card, Checkbox, Chip, Empty, Modal, Page, Skeleton, SoftTile, cx } from "../components/ui";
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

// Colours for the "what's inside" breakdown in the side panel.
const INSIDE = ["#22c55e", "#3b82f6", "#8b5cf6", "#f59e0b", "#ec4899"];

const AGE_FILTERS = [
  { label: "Any age", days: 0 },
  { label: "Not changed in 3 months", days: 90 },
  { label: "Not changed in 6 months", days: 180 },
  { label: "Not changed in a year", days: 365 },
];

const selectClass = "glass h-8 cursor-pointer rounded-[9px] px-2 text-[12.5px]";

export default function LargeFiles() {
  const { files, setFiles, home, deleteMode, reportDelete, toast } = useStore();
  const [scanning, setScanning] = useState(false);
  const [tab, setTab] = useState<"folders" | "files">("folders");
  const [cwd, setCwd] = useState<string | null>(files?.root ?? null);
  const [nodes, setNodes] = useState<Node[]>([]);
  const [focus, setFocus] = useState<Node | null>(null);
  const [query, setQuery] = useState("");
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
    if (cwd && files)
      api
        .bigfilesChildren(cwd)
        .then((n) => {
          setNodes(n);
          setFocus(n[0] ?? null);
        })
        .catch(() => setNodes([]));
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

  const q = query.trim().toLowerCase();
  const shownNodes = q ? nodes.filter((n) => n.name.toLowerCase().includes(q)) : nodes;
  const topFiles = useMemo(() => {
    const cutoff = Date.now() / 1000 - age * 86400;
    return (files?.top_files ?? []).filter(
      (f) => (kind === "all" || f.kind === kind) && (age === 0 || (f.modified ?? 0) < cutoff) && (!q || f.name.toLowerCase().includes(q)),
    );
  }, [files, kind, age, q]);

  const parentTotal = nodes.reduce((a, n) => a + n.bytes, 0);

  return (
    <Page
      title="Files"
      subtitle="See which folders and files take the most space in your home folder, then pick what to remove."
      actions={
        <>
          {files && !scanning && (
            <label className="glass flex h-8 w-56 items-center gap-2 rounded-[9px] px-2.5">
              <Search className="size-3.5 text-faint" aria-hidden />
              <input
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="Search files and folders…"
                aria-label="Search files and folders"
                className="w-full bg-transparent text-[12.5px] outline-none placeholder:text-faint"
              />
            </label>
          )}
          {scanning ? (
            <Button onClick={() => api.cancelScan()}>
              <StopCircle className="size-3.5" aria-hidden /> Stop
            </Button>
          ) : (
            <Button variant={files ? "secondary" : "primary"} onClick={scan}>
              <FolderSearch className="size-3.5" aria-hidden /> {files ? "Rescan" : "Scan home folder"}
            </Button>
          )}
        </>
      }
    >
      {scanning && (
        <Card className="p-5">
          <p className="mb-3 text-[13px] text-muted" aria-live="polite">
            Scanning… <span className="tabular font-medium text-ink">{(progress?.files ?? 0).toLocaleString()}</span> files,{" "}
            <span className="tabular font-medium text-ink">{bytes(progress?.bytes ?? 0)}</span>
          </p>
          <Skeleton rows={7} />
        </Card>
      )}

      {!files && !scanning && (
        <Card>
          <Empty icon={<FolderSearch className="size-7" aria-hidden />} title="Find what's using your space">
            Walks your whole home folder (usually under a minute) and shows the biggest folders and files. Nothing is changed until you choose.
          </Empty>
        </Card>
      )}

      {files && !scanning && (
        <>
          {/* What kind of files fill the home folder. */}
          <Card className="mb-4 px-5 py-4">
            <div className="mb-2 flex items-baseline justify-between text-[13px]">
              <span className="font-semibold">
                {bytes(files.total_bytes)} in {files.file_count.toLocaleString()} files
              </span>
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

          <div className="mb-3 flex flex-wrap items-center gap-2">
            <Chip active={tab === "folders"} onClick={() => setTab("folders")}>
              Browse folders
            </Chip>
            <Chip active={tab === "files"} onClick={() => setTab("files")}>
              Biggest files
            </Chip>
            {tab === "files" && (
              <>
                <select className={selectClass} aria-label="File type" value={kind} onChange={(e) => setKind(e.target.value as Kind | "all")}>
                  <option value="all">All types</option>
                  {Object.entries(KIND).map(([k, v]) => (
                    <option key={k} value={k}>
                      {v.label}
                    </option>
                  ))}
                </select>
                <select className={selectClass} aria-label="Last changed" value={age} onChange={(e) => setAge(Number(e.target.value))}>
                  {AGE_FILTERS.map((a) => (
                    <option key={a.days} value={a.days}>
                      {a.label}
                    </option>
                  ))}
                </select>
              </>
            )}
            <div className="flex-1" />
            {sel.size > 0 && (
              <>
                <span className="tabular text-[12.5px] text-muted">
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
            <div className="grid grid-cols-[minmax(0,1fr)_300px] items-start gap-5">
              <Card className="overflow-hidden">
                <div className="flex flex-wrap items-center gap-1 border-b border-line px-4 py-2.5 text-[13px]">
                  {crumbs.map((c, i) => (
                    <span key={c.path} className="flex items-center gap-1">
                      {i > 0 && <ChevronRight className="size-3.5 text-faint" />}
                      <button onClick={() => setCwd(c.path)} className={cx("cursor-pointer rounded px-1 hover:bg-ink/[0.04]", i === crumbs.length - 1 ? "font-semibold" : "text-muted")}>
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
                  {shownNodes.length === 0 && <li className="p-6 text-center text-[13px] text-muted">{q ? "Nothing here matches your search." : "Empty folder"}</li>}
                  {shownNodes.slice(0, 300).map((n) => (
                    <li
                      key={n.path}
                      className={cx("group flex cursor-pointer items-center gap-3 px-4 py-2.5 text-[13px] transition-colors", focus?.path === n.path ? "bg-accent-soft" : "hover:bg-ink/[0.03]")}
                      onClick={() => setFocus(n)}
                      onDoubleClick={() => n.is_dir && setCwd(n.path)}
                    >
                      <Checkbox label={n.name} checked={sel.has(n.path)} disabled={isProtected(n.path, n.is_dir, home)} onChange={(on) => toggle(n.path, n.bytes, on)} />
                      {n.is_dir ? <FolderClosed className="size-[18px] shrink-0 fill-info/25 text-info" aria-hidden /> : <File className="size-[18px] shrink-0 text-faint" aria-hidden />}
                      <span className="w-52 truncate font-medium" title={n.path}>
                        {n.name}
                      </span>
                      <div className="h-1.5 flex-1 overflow-hidden rounded-full bg-ink/[0.06]">
                        <div className="h-full rounded-full bg-gradient-to-r from-[#60a5fa] to-[#2563eb] transition-[width] duration-500" style={{ width: `${percent(n.bytes, parentTotal)}%` }} />
                      </div>
                      <span className="tabular w-20 text-right font-semibold">{bytes(n.bytes)}</span>
                      {n.is_dir ? (
                        <button
                          title="Open folder"
                          aria-label={`Open ${n.name}`}
                          className="cursor-pointer rounded-md p-0.5 text-faint hover:bg-ink/[0.05] hover:text-ink"
                          onClick={(e) => {
                            e.stopPropagation();
                            setCwd(n.path);
                          }}
                        >
                          <ChevronRight className="size-4" />
                        </button>
                      ) : (
                        <span className="w-5" />
                      )}
                    </li>
                  ))}
                </ul>
              </Card>

              <FolderPanel
                node={focus}
                share={focus ? percent(focus.bytes, parentTotal) : 0}
                home={home}
                selected={!!focus && sel.has(focus.path)}
                onOpen={() => focus?.is_dir && setCwd(focus.path)}
                onSelect={(on) => focus && toggle(focus.path, focus.bytes, on)}
              />
            </div>
          ) : (
            <Card className="overflow-hidden">
              <div className="border-b border-line px-4 py-2 text-[11.5px] text-faint">Files over 10 MB</div>
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

/** Details of the highlighted folder or file: size, share, and what's inside. */
function FolderPanel({ node, share, home, selected, onOpen, onSelect }: { node: Node | null; share: number; home: string | null; selected: boolean; onOpen: () => void; onSelect: (on: boolean) => void }) {
  const [inside, setInside] = useState<Node[] | null>(null);
  useEffect(() => {
    setInside(null);
    if (node?.is_dir) api.bigfilesChildren(node.path).then(setInside).catch(() => setInside([]));
  }, [node]);

  if (!node)
    return (
      <Card className="p-5 text-center text-[12.5px] text-muted">
        <FolderOpen className="mx-auto mb-2 size-6 text-faint" aria-hidden />
        Pick a folder to see what's inside.
      </Card>
    );

  const locked = isProtected(node.path, node.is_dir, home);
  const top = (inside ?? []).slice(0, INSIDE.length);
  const rest = (inside ?? []).slice(INSIDE.length).reduce((a, n) => a + n.bytes, 0);
  const parts = [...top.map((n, i) => ({ name: n.name, bytes: n.bytes, color: INSIDE[i] })), ...(rest > 0 ? [{ name: "Other", bytes: rest, color: "var(--c-faint)" }] : [])];
  const total = parts.reduce((a, p) => a + p.bytes, 0);

  return (
    <Card className="sticky top-4 p-5">
      <div className="flex items-center gap-3">
        <SoftTile icon={node.is_dir ? FolderClosed : File} color="var(--c-info)" size={50} />
        <div className="min-w-0">
          <div className="truncate text-[17px] font-semibold" title={node.path}>
            {node.name}
          </div>
          <div className="text-[12.5px] text-muted">
            {bytes(node.bytes)} · {Math.round(share)}% of this folder
          </div>
        </div>
      </div>
      <div className="selectable mt-2 truncate font-mono text-[11px] text-faint" title={node.path}>
        {tildify(node.path, home)}
      </div>

      {node.is_dir && (
        <div className="mt-4">
          <div className="mb-1.5 text-[12px] font-semibold text-muted">What's inside</div>
          {inside === null ? (
            <div className="skeleton h-3" />
          ) : parts.length === 0 ? (
            <p className="text-[12px] text-faint">Empty folder.</p>
          ) : (
            <>
              <div className="flex h-3 overflow-hidden rounded-full bg-ink/[0.06]">
                {parts.map((p) => (
                  <div key={p.name} style={{ width: `${percent(p.bytes, total)}%`, background: p.color }} title={`${p.name}: ${bytes(p.bytes)}`} />
                ))}
              </div>
              <ul className="mt-3 space-y-1.5 text-[12.5px]">
                {parts.map((p) => (
                  <li key={p.name} className="flex items-center gap-2">
                    <span className="size-2 shrink-0 rounded-full" style={{ background: p.color }} />
                    <span className="min-w-0 flex-1 truncate text-muted">{p.name}</span>
                    <span className="tabular font-medium">{bytes(p.bytes)}</span>
                  </li>
                ))}
              </ul>
            </>
          )}
        </div>
      )}

      {locked && <p className="mt-4 rounded-[10px] bg-warn-soft px-3 py-2 text-[12px] text-warn-text">Protected: apps or macOS need this, so Mr.Clean won't remove it.</p>}

      <div className="mt-4 flex flex-col gap-2">
        {node.is_dir && (
          <button
            onClick={onOpen}
            className="flex h-10 cursor-pointer items-center justify-center gap-2 rounded-[12px] border border-white/40 bg-gradient-to-br from-[#4ade80] to-[#16a34a] text-[13.5px] font-semibold text-white shadow-[inset_0_1px_0_rgba(255,255,255,0.45),0_10px_22px_-12px_rgba(22,163,74,0.8)]"
          >
            <FolderOpen className="size-4" aria-hidden /> Review folder
          </button>
        )}
        <div className="flex gap-2">
          <Button className="flex-1" onClick={() => api.reveal(node.path)}>
            <ExternalLink className="size-3.5" aria-hidden /> Show in Finder
          </Button>
          {!locked && (
            <Button className="flex-1" variant={selected ? "danger" : "secondary"} onClick={() => onSelect(!selected)}>
              <Trash2 className="size-3.5" aria-hidden /> {selected ? "Selected" : "Select"}
            </Button>
          )}
        </div>
      </div>
    </Card>
  );
}

function FileRow({ f, home, checked, onToggle }: { f: FileEntry; home: string | null; checked: boolean; onToggle: (on: boolean) => void }) {
  return (
    <li className="group flex items-center gap-3 px-4 py-2.5 text-[13px] hover:bg-ink/[0.03]">
      <Checkbox label={f.name} checked={checked} disabled={isProtected(f.path, false, home)} onChange={onToggle} />
      <SoftTile icon={File} color={KIND[f.kind].color} size={32} />
      <div className="min-w-0 flex-1">
        <div className="truncate font-medium">{f.name}</div>
        <div className="selectable truncate text-[11.5px] text-faint" title={f.path}>
          {tildify(f.path, home)}
        </div>
      </div>
      <Badge>{KIND[f.kind].label}</Badge>
      <span className="w-28 text-right text-[11.5px] text-faint">{ago(f.modified)}</span>
      <button
        title="Show in Finder"
        className="cursor-pointer rounded p-1 text-faint opacity-0 transition hover:bg-ink/[0.05] hover:text-ink group-hover:opacity-100"
        onClick={() => api.reveal(f.path)}
      >
        <ExternalLink className="size-3.5" />
      </button>
      <span className="tabular w-20 text-right font-semibold">{bytes(f.bytes)}</span>
    </li>
  );
}
