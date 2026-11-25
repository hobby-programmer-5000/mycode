#!/usr/bin/env node
import * as NodeFS from "node:fs";
import * as NodePath from "node:path";
import * as NodeURL from "node:url";

const repoRoot = NodePath.resolve(NodeURL.fileURLToPath(new URL("..", import.meta.url)));
const srcDir = NodePath.join(repoRoot, "apps", "web", "dist");
const destDir = NodePath.join(repoRoot, "apps", "desktop-tauri", "dist-web");

if (!NodeFS.existsSync(srcDir)) {
  throw new Error(`Missing web build at ${srcDir}. Run \`pnpm build\` first.`);
}

NodeFS.rmSync(destDir, { recursive: true, force: true });
let copiedFiles = 0;
let skippedMaps = 0;
let totalBytes = 0;

function copyFiltered(from, to) {
  NodeFS.mkdirSync(to, { recursive: true });
  for (const entry of NodeFS.readdirSync(from, { withFileTypes: true })) {
    const src = NodePath.join(from, entry.name);
    const dst = NodePath.join(to, entry.name);
    if (entry.isDirectory()) {
      copyFiltered(src, dst);
    } else if (entry.isFile()) {
      if (entry.name.endsWith(".map")) {
        skippedMaps += 1;
        continue;
      }
      NodeFS.copyFileSync(src, dst);
      copiedFiles += 1;
      totalBytes += NodeFS.statSync(dst).size;
    }
  }
}

copyFiltered(srcDir, destDir);
console.log(
  `[tauri-web] Staged ${copiedFiles} files (${(totalBytes / 1024 / 1024).toFixed(1)} MB) to ${destDir}; excluded ${skippedMaps} .map files.`,
);
