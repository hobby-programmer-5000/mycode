// @effect-diagnostics nodeBuiltinImport:off globalTimers:off
import * as NodeStringDecoder from "node:string_decoder";

import * as Effect from "effect/Effect";
import * as Layer from "effect/Layer";
import * as PtyAdapter from "@mycode/shared/PtyAdapter";

import { loadRustNativeAddon, type MyCodeRustNativeAddon } from "../native/rustNative.ts";

class RustPtyProcess implements PtyAdapter.PtyProcess {
  readonly pid: number;
  private readonly addon: MyCodeRustNativeAddon;
  private readonly sessionId: string;
  private readonly dataListeners = new Set<(data: string) => void>();
  private readonly exitListeners = new Set<(event: PtyAdapter.PtyExitEvent) => void>();
  private readonly decoder = new NodeStringDecoder.StringDecoder("utf8");
  private timer: ReturnType<typeof setInterval> | null = null;
  private closed = false;

  constructor(addon: MyCodeRustNativeAddon, sessionId: string, pid: number) {
    this.addon = addon;
    this.sessionId = sessionId;
    this.pid = pid;
    this.timer = setInterval(() => this.pollOnce(), 16);
    this.timer.unref?.();
  }

  private pollOnce(): void {
    if (this.closed) return;
    let res;
    try {
      res = this.addon.ptyPoll(this.sessionId);
    } catch {
      this.finish(0);
      return;
    }
    if (res.data.length > 0) {
      const chunk = this.decoder.write(res.data);
      if (chunk.length > 0) {
        for (const listener of this.dataListeners) {
          listener(chunk);
        }
      }
    }
    if (res.exited) {
      const tail = this.decoder.end();
      if (tail.length > 0) {
        for (const listener of this.dataListeners) {
          listener(tail);
        }
      }
      this.finish(res.exitCode ?? 0);
    }
  }

  private finish(exitCode: number): void {
    if (this.closed) return;
    this.closed = true;
    if (this.timer !== null) {
      clearInterval(this.timer);
      this.timer = null;
    }
    for (const listener of this.exitListeners) {
      listener({ exitCode, signal: null });
    }
    this.dataListeners.clear();
    this.exitListeners.clear();
  }

  write(data: string | Buffer): void {
    if (this.closed) return;
    const buf = typeof data === "string" ? Buffer.from(data, "utf8") : data;
    this.addon.ptyWrite(this.sessionId, buf);
  }

  resize(cols: number, rows: number): void {
    if (this.closed) return;
    this.addon.ptyResize(this.sessionId, cols, rows);
  }

  kill(_signal?: string): void {
    if (this.closed) return;
    try {
      this.addon.ptyKill(this.sessionId);
    } catch {
      // Already exited.
    }
    this.finish(0);
  }

  onData(callback: (data: string) => void): () => void {
    this.dataListeners.add(callback);
    return () => {
      this.dataListeners.delete(callback);
    };
  }

  onExit(callback: (event: PtyAdapter.PtyExitEvent) => void): () => void {
    this.exitListeners.add(callback);
    return () => {
      this.exitListeners.delete(callback);
    };
  }
}

/**
 * Rust ConPTY / Unix PTY implementation of `PtyAdapter` backed by `crates/mycode-native`
 * (`portable-pty`).
 */
export const layer = Layer.succeed(
  PtyAdapter.PtyAdapter,
  PtyAdapter.PtyAdapter.of({
    spawn: Effect.fn("RustPtyAdapter.spawn")(function* (input: PtyAdapter.PtySpawnInput) {
      return yield* Effect.try({
        try: () => {
          const addon = loadRustNativeAddon();
          if (addon === null) {
            throw new Error("mycode_native.node is not built or could not be loaded.");
          }
          const env: Record<string, string> = {};
          for (const [k, v] of Object.entries(input.env)) {
            if (v !== undefined) env[k] = v;
          }
          const info = addon.ptySpawn({
            shell: input.shell,
            args: input.args ?? [],
            cwd: input.cwd,
            cols: input.cols,
            rows: input.rows,
            env,
          });
          return new RustPtyProcess(addon, info.sessionId, info.pid ?? 0);
        },
        catch: (cause) =>
          new PtyAdapter.PtySpawnError({
            adapter: "rust-portable-pty",
            shell: input.shell,
            cause,
          }),
      });
    }),
  }),
);
