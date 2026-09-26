// Copies the few brand logos we use from simple-icons (CC0) into the app,
// plus their brand colours. Run after changing the list:
//   node scripts/copy-tool-icons.mjs
import { copyFileSync, mkdirSync, writeFileSync } from "node:fs";
import * as si from "simple-icons";

const root = new URL("../", import.meta.url).pathname;
const src = `${root}node_modules/simple-icons/icons/`;
const out = `${root}src/assets/tool-icons/`;

const TOOLS = [
  "npm", "yarn", "pnpm", "bun", "homebrew", "rust", "python", "gradle", "android", "androidstudio",
  "xcode", "apple", "docker", "flutter", "dart", "cocoapods", "go", "jetbrains", "swift", "electron",
  "nodedotjs", "deno", "expo", "unity", "googlechrome", "cursor", "kotlin",
  "dotnet", "nuget", "ubuntu", "debian", "linux", "flatpak", "snapcraft",
];

const bySlug = Object.fromEntries(Object.values(si).map((i) => [i.slug, i]));
mkdirSync(out, { recursive: true });
const colors = {};
for (const t of TOOLS) {
  copyFileSync(`${src}${t}.svg`, `${out}${t}.svg`);
  colors[t] = `#${bySlug[t].hex}`;
}
writeFileSync(`${out}colors.json`, JSON.stringify(colors, null, 2) + "\n");
console.log(`copied ${TOOLS.length} icons`);
