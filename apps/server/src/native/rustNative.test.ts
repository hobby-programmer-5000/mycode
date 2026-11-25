// @effect-diagnostics nodeBuiltinImport:off globalDate:off globalTimers:off
import * as NodeFS from "node:fs";
import * as NodeOS from "node:os";
import * as NodePath from "node:path";

import { assert, describe, it } from "@effect/vitest";

import { loadRustNativeAddon } from "./rustNative.ts";

describe("mycode-native Rust N-API addon", () => {
  it("loads and runs FNV-1a, offline firewall, workspace search, transcript walk, git summary, and ConPTY", async () => {
    const addon = loadRustNativeAddon();
    assert.isNotNull(addon, "expected compiled apps/server/native/mycode_native.node");
    if (addon === null) return;

    // 1. FNV-1a exact compatibility check
    assert.strictEqual(addon.fnv1aHash(Buffer.from("")), 0x811c9dc5);
    assert.strictEqual(addon.fnv1aHash(Buffer.from("hello")), 0x4f9f2cab);
    assert.strictEqual(addon.fnv1aHash(Buffer.from("mycode")), 0xcffde70a);

    // 2. Offline loopback firewall check
    assert.isTrue(addon.isAllowedOfflineUrl("http://127.0.0.1:3773/.well-known/mycode/environment"));
    assert.isTrue(addon.isAllowedOfflineUrl("http://tauri.localhost/index.html"));
    assert.isFalse(addon.isAllowedOfflineUrl("https://github.com/hobby-programmer-5000/mycode"));
    assert.isFalse(addon.isAllowedOfflineUrl("http://192.168.1.5:3773/api/health"));

    // 3. Workspace listing & content search respecting .gitignore + transcript walker
    const tmpDir = NodeFS.mkdtempSync(NodePath.join(NodeOS.tmpdir(), "mycode-rust-napi-"));
    try {
      NodeFS.mkdirSync(NodePath.join(tmpDir, ".git"));
      NodeFS.mkdirSync(NodePath.join(tmpDir, "src"));
      NodeFS.mkdirSync(NodePath.join(tmpDir, "ignored"));
      NodeFS.writeFileSync(NodePath.join(tmpDir, ".gitignore"), "ignored/\n");
      NodeFS.writeFileSync(NodePath.join(tmpDir, "src", "main.ts"), "export const name = 'MyCode';\n");
      NodeFS.writeFileSync(NodePath.join(tmpDir, "ignored", "secret.ts"), "MyCode secret\n");
      NodeFS.writeFileSync(NodePath.join(tmpDir, "session.jsonl"), '{"usage":1}\n');

      const entries = addon.listWorkspaceEntries(tmpDir, 100, false);
      assert.isTrue(entries.entries.some((e) => e.path === "src/main.ts" && e.kind === "file"));
      assert.isFalse(entries.entries.some((e) => e.path.startsWith("ignored")));

      const search = addon.searchWorkspaceContents(tmpDir, "mycode", false, 50, 10);
      assert.strictEqual(search.matches.length, 1);
      assert.strictEqual(search.matches[0]?.path, "src/main.ts");
      assert.strictEqual(search.matches[0]?.line, 1);

      const walked = addon.listTranscriptFiles(tmpDir, 0, null);
      assert.strictEqual(walked.failedPaths, 0);
      assert.strictEqual(walked.files.length, 1);
      assert.isTrue(walked.files[0]?.path.endsWith("session.jsonl"));
    } finally {
      NodeFS.rmSync(tmpDir, { recursive: true, force: true });
    }

    // 4. Git summary against the live repo root
    const repoRoot = NodePath.resolve(process.cwd(), "..", "..");
    const gitSummary = addon.gitRepoSummary(repoRoot);
    assert.isTrue(gitSummary.isRepo);
    assert.isString(gitSummary.branch);

    // 5. Native Rust PTY round-trip
    const spawned = addon.ptySpawn(
      process.platform === "win32"
        ? {
            shell: "cmd.exe",
            args: ["/C", "echo NAPI_RUST_PTY_OK"],
            cwd: NodeOS.tmpdir(),
            cols: 80,
            rows: 24,
          }
        : {
            shell: "/bin/sh",
            args: ["-c", "echo NAPI_RUST_PTY_OK"],
            cwd: NodeOS.tmpdir(),
            cols: 80,
            rows: 24,
          },
    );
    let output = "";
    let answeredDsr = false;
    const deadline = Date.now() + 5_000;
    while (Date.now() < deadline) {
      const polled = addon.ptyPoll(spawned.sessionId);
      if (polled.data.length > 0) output += polled.data.toString("utf8");
      if (!answeredDsr && output.includes("\x1b[6n")) {
        addon.ptyWrite(spawned.sessionId, Buffer.from("\x1b[1;1R", "utf8"));
        answeredDsr = true;
      }
      if (output.includes("NAPI_RUST_PTY_OK")) break;
      await new Promise((r) => setTimeout(r, 30));
    }
    addon.ptyKill(spawned.sessionId);
    assert.include(output, "NAPI_RUST_PTY_OK");
  });
});
