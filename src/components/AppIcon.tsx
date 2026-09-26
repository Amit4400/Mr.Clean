import { useEffect, useState } from "react";

import colors from "../assets/tool-icons/colors.json";
import { api } from "../lib/api";
import { cx } from "./ui";

// Bundled brand logos from simple-icons (CC0), keyed by slug.
const toolUrls = Object.fromEntries(
  Object.entries(import.meta.glob("../assets/tool-icons/*.svg", { eager: true, query: "?url", import: "default" })).map(([k, v]) => [
    k.split("/").pop()!.replace(".svg", ""),
    v as string,
  ]),
);
const toolColors = colors as Record<string, string>;

// Process and app names on Windows/Linux that map to a bundled logo.
const ALIASES: Record<string, string> = {
  nodejs: "nodedotjs",
  node: "nodedotjs",
  chrome: "googlechrome",
  dockerdesktop: "docker",
  intellijidea: "jetbrains",
  idea64: "jetbrains",
  studio64: "androidstudio",
  gradledaemon: "gradle",
};

export const hasToolIcon = (slug: string) => slug in toolUrls;

// App icons are read once per app and kept for the session.
const iconCache = new Map<string, Promise<string | null>>();
function loadIcon(key: string, q: { bundle?: string | null; name?: string }) {
  if (!iconCache.has(key)) iconCache.set(key, api.appIcon(q).catch(() => null));
  return iconCache.get(key)!;
}

/** Stable pleasant colour for a name (for the letter fallback). */
function hue(name: string) {
  let h = 0;
  for (const c of name) h = (h * 31 + c.charCodeAt(0)) % 360;
  return `hsl(${h} 55% 52%)`;
}

/**
 * The icon for an app or developer tool:
 * 1. the real app icon from the installed `.app` (macOS),
 * 2. else a bundled brand logo (npm, Homebrew, Rust…),
 * 3. else a tinted letter tile.
 */
export function AppIcon({ name, bundle, app, tool, size = 32, className }: { name: string; bundle?: string | null; app?: string; tool?: string; size?: number; className?: string }) {
  const key = bundle ?? (app ? `name:${app}` : "");
  const [src, setSrc] = useState<string | null>(null);
  useEffect(() => {
    let live = true;
    setSrc(null);
    if (key) loadIcon(key, bundle ? { bundle } : { name: app }).then((s) => live && setSrc(s));
    return () => {
      live = false;
    };
  }, [key, bundle, app]);

  const box = { width: size, height: size };
  if (src) return <img src={src} alt="" className={cx("shrink-0", className)} style={box} draggable={false} />;

  // Known apps without a readable icon fall back to their brand logo.
  const plain = name.toLowerCase().replace(/[^a-z0-9]/g, "");
  const slug = tool ?? ALIASES[plain] ?? plain;
  if (toolUrls[slug]) tool = slug;
  if (tool && toolUrls[tool]) {
    const color = toolColors[tool] === "#000000" ? "var(--c-ink)" : toolColors[tool];
    return (
      <span
        className={cx("flex shrink-0 items-center justify-center rounded-[25%]", className)}
        style={{ ...box, background: `color-mix(in srgb, ${color} 13%, transparent)` }}
        aria-hidden
      >
        <span
          style={{
            width: size * 0.56,
            height: size * 0.56,
            background: color,
            WebkitMask: `url("${toolUrls[tool]}") center / contain no-repeat`,
            mask: `url("${toolUrls[tool]}") center / contain no-repeat`,
          }}
        />
      </span>
    );
  }

  const c = hue(name);
  return (
    <span
      className={cx("flex shrink-0 items-center justify-center rounded-[25%] font-semibold", className)}
      style={{ ...box, color: c, background: `color-mix(in srgb, ${c} 14%, transparent)`, fontSize: size * 0.42 }}
      aria-hidden
    >
      {name.trim().charAt(0).toUpperCase() || "?"}
    </span>
  );
}
