// @effect-diagnostics nodeBuiltinImport:off
import * as NodeFS from "node:fs";
import * as NodeModule from "node:module";
import * as NodePath from "node:path";
import * as NodeURL from "node:url";

export interface NativeTranscriptFile {
  readonly path: string;
  readonly size: number;
  readonly mtimeMs: number;
}

export interface NativeTranscriptWalkResult {
  readonly files: readonly NativeTranscriptFile[];
  readonly walked: readonly string[];
  readonly failedPaths: number;
}

export interface NativeWorkspaceEntry {
  readonly path: string;
  readonly kind: "file" | "directory";
}

export interface NativeWorkspaceListResult {
  readonly entries: readonly NativeWorkspaceEntry[];
  readonly truncated: boolean;
}

export interface NativeContentSearchMatch {
  readonly path: string;
  readonly line: number;
  readonly column: number;
  readonly lineText: string;
}

export interface NativeContentSearchResult {
  readonly matches: readonly NativeContentSearchMatch[];
  readonly truncated: boolean;
  readonly filesScanned: number;
}

export interface NativeGitRepoSummary {
  readonly isRepo: boolean;
  readonly branch: string | null;
  readonly headOid: string | null;
  readonly upstream: string | null;
  readonly ahead: number;
  readonly behind: number;
  readonly stagedCount: number;
  readonly unstagedCount: number;
  readonly untrackedCount: number;
  readonly hasWorkingTreeChanges: boolean;
}

export interface NativePtySpawnOptions {
  readonly shell: string;
  readonly args: string[];
  readonly cwd: string;
  readonly cols: number;
  readonly rows: number;
  readonly env?: Record<string, string>;
}

export interface NativePtySpawnInfo {
  readonly sessionId: string;
  readonly pid?: number | null;
}

export interface NativePtyPollResult {
  readonly data: Buffer;
  readonly exited: boolean;
  readonly exitCode?: number | null;
}

export interface MyCodeRustNativeAddon {
  readonly fnv1aHash: (buf: Buffer) => number;
  readonly listTranscriptFiles: (
    root: string,
    sinceMs: number,
    fileName?: string | null,
  ) => NativeTranscriptWalkResult;
  readonly listWorkspaceEntries: (
    root: string,
    maxEntries: number,
    includeHidden: boolean,
  ) => NativeWorkspaceListResult;
  readonly searchWorkspaceContents: (
    root: string,
    query: string,
    caseSensitive: boolean,
    maxMatches: number,
    maxMatchesPerFile: number,
  ) => NativeContentSearchResult;
  readonly gitRepoSummary: (cwd: string) => NativeGitRepoSummary;
  readonly isAllowedOfflineUrl: (url: string) => boolean;
  readonly ptySpawn: (options: NativePtySpawnOptions) => NativePtySpawnInfo;
  readonly ptyWrite: (sessionId: string, data: Buffer) => void;
  readonly ptyResize: (sessionId: string, cols: number, rows: number) => void;
  readonly ptyPoll: (sessionId: string) => NativePtyPollResult;
  readonly ptyKill: (sessionId: string) => void;
}

let cachedAddon: MyCodeRustNativeAddon | null | undefined;

function resolveAddonCandidates(): string[] {
  const explicit = process.env.MYCODE_RUST_NATIVE_PATH?.trim();
  const here = NodePath.dirname(NodeURL.fileURLToPath(import.meta.url));
  const candidates = [
    ...(explicit ? [explicit] : []),
    NodePath.resolve(here, "../../native/mycode_native.node"),
    NodePath.resolve(here, "../native/mycode_native.node"),
    NodePath.resolve(process.cwd(), "apps/server/native/mycode_native.node"),
    NodePath.resolve(process.cwd(), "native/mycode_native.node"),
  ];
  return candidates;
}

/**
 * Loads the compiled Rust N-API addon (`crates/mycode-native` -> `mycode_native.node`) when
 * present on disk, returning `null` cleanly so pure-TS fallbacks work in environments without
 * compiled native artifacts.
 */
export function loadRustNativeAddon(): MyCodeRustNativeAddon | null {
  if (cachedAddon !== undefined) return cachedAddon;
  const req = NodeModule.createRequire(import.meta.url);
  for (const candidate of resolveAddonCandidates()) {
    if (!NodeFS.existsSync(candidate)) continue;
    try {
      cachedAddon = req(candidate) as MyCodeRustNativeAddon;
      return cachedAddon;
    } catch {
      // Try next candidate path.
    }
  }
  cachedAddon = null;
  return null;
}
