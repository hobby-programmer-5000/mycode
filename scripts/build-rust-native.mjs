#!/usr/bin/env node
import * as NodeChildProcess from "node:child_process";
import * as NodeFS from "node:fs";
import * as NodePath from "node:path";
import * as NodeURL from "node:url";

const repoRoot = NodePath.resolve(NodeURL.fileURLToPath(new URL("..", import.meta.url)));
const cargoPath = process.env.CARGO || (
  process.platform === "win32"
    ? NodePath.join(process.env.USERPROFILE || "", ".cargo", "bin", "cargo.exe")
    : "cargo"
);
const cargoCmd = NodeFS.existsSync(cargoPath) ? cargoPath : "cargo";

const result = NodeChildProcess.spawnSync(
  cargoCmd,
  ["build", "--release", "-p", "mycode-native"],
  {
    cwd: repoRoot,
    stdio: "inherit",
  },
);
if (result.status !== 0) {
  process.exit(result.status ?? 1);
}

const artifactName =
  process.platform === "win32"
    ? "mycode_native.dll"
    : process.platform === "darwin"
      ? "libmycode_native.dylib"
      : "libmycode_native.so";

const source = NodePath.join(repoRoot, "target", "release", artifactName);
const destDir = NodePath.join(repoRoot, "apps", "server", "native");
const dest = NodePath.join(destDir, "mycode_native.node");

NodeFS.mkdirSync(destDir, { recursive: true });
NodeFS.copyFileSync(source, dest);
console.log(`[rust-native] Built and staged ${dest} (${NodeFS.statSync(dest).size} bytes)`);
