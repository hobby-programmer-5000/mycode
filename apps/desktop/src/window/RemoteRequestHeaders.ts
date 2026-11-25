import * as NetAddress from "effect/net/NetAddress";
import * as Result from "effect/Result";
import type * as Electron from "electron";

// `*://` matches only http and https, so WebSocket schemes are listed separately.
const REMOTE_REQUEST_URL_PATTERNS = ["http://*/*", "https://*/*", "ws://*/*", "wss://*/*"];

export interface MainWindowRequestHeaderPolicy {
  readonly desktopOrigin: string;
  readonly mainWebContentsId: number;
  readonly clerkFrontendApiHostname?: string | undefined;
  readonly devServerOrigin?: string | undefined;
  readonly isDevelopment: boolean;
}

export interface MainWindowRequestHeaderDetails {
  readonly url: string;
  readonly webContentsId?: number | undefined;
  readonly frame?: { readonly parent: unknown } | null | undefined;
  readonly requestHeaders: Record<string, string>;
}

function normalizeHostname(hostname: string): string {
  return hostname
    .toLowerCase()
    .replace(/^\[(.*)\]$/, "$1")
    .replace(/\.$/, "");
}

function isLoopbackHostname(hostname: string): boolean {
  return (
    hostname === "localhost" ||
    hostname.endsWith(".localhost") ||
    Result.match(NetAddress.ipFromString(hostname), {
      onSuccess: NetAddress.isLoopback,
      onFailure: () => false,
    })
  );
}

function toHttpOrigin(url: URL): string {
  if (url.protocol === "ws:") {
    return `http://${url.host}`;
  }
  if (url.protocol === "wss:") {
    return `https://${url.host}`;
  }
  return url.origin;
}

function isSubframe(details: MainWindowRequestHeaderDetails): boolean {
  try {
    return details.frame != null && details.frame.parent !== null;
  } catch {
    return true;
  }
}

function hasExactDesktopOriginHeader(
  requestHeaders: Record<string, string>,
  desktopOrigin: string,
): boolean {
  let matched = false;
  for (const [name, value] of Object.entries(requestHeaders)) {
    if (name.toLowerCase() !== "origin") continue;
    if (value !== desktopOrigin) return false;
    matched = true;
  }
  return matched;
}

function shouldPreserveOriginHeader(policy: MainWindowRequestHeaderPolicy, url: URL): boolean {
  const hostname = normalizeHostname(url.hostname);
  if (isLoopbackHostname(hostname)) {
    return true;
  }
  if (!policy.isDevelopment) return false;
  if (policy.devServerOrigin !== undefined && toHttpOrigin(url) === policy.devServerOrigin) {
    return true;
  }
  return false;
}

/**
 * Offline local-only firewall: blocks any non-loopback HTTP(S)/WS(S) request originating from the
 * Electron renderer session so the app itself only talks to its local loopback engine.
 * Spawned OS CLI processes (Claude Code, Codex, Antigravity) run outside Electron's network stack.
 */
export function shouldBlockMainWindowExternalRequest(
  policy: MainWindowRequestHeaderPolicy,
  details: Pick<MainWindowRequestHeaderDetails, "url" | "webContentsId">,
): boolean {
  if (details.webContentsId !== undefined && details.webContentsId !== policy.mainWebContentsId) {
    return false;
  }
  const parsedUrl = URL.parse(details.url);
  if (parsedUrl === null) return true;
  const hostname = normalizeHostname(parsedUrl.hostname);
  if (isLoopbackHostname(hostname)) return false;
  if (
    policy.isDevelopment &&
    policy.devServerOrigin !== undefined &&
    toHttpOrigin(parsedUrl) === policy.devServerOrigin
  ) {
    return false;
  }
  return true;
}

/**
 * Preserves Electron's synthetic `Origin` (`mycode://app`) for loopback backend requests (required
 * for CORS preflight) and strips it on any non-loopback requests.
 */
export function stripDesktopOriginHeaders(
  policy: MainWindowRequestHeaderPolicy,
  details: MainWindowRequestHeaderDetails,
): Record<string, string> | undefined {
  if (details.webContentsId !== policy.mainWebContentsId) return undefined;
  if (isSubframe(details)) return undefined;
  if (!hasExactDesktopOriginHeader(details.requestHeaders, policy.desktopOrigin)) {
    return undefined;
  }

  const parsedUrl = URL.parse(details.url);
  if (parsedUrl === null || shouldPreserveOriginHeader(policy, parsedUrl)) {
    return undefined;
  }

  const nextHeaders: Record<string, string> = {};
  for (const [name, value] of Object.entries(details.requestHeaders)) {
    const lowerName = name.toLowerCase();
    if (lowerName === "origin" || lowerName.startsWith("sec-fetch-")) continue;
    nextHeaders[name] = value;
  }
  return nextHeaders;
}

export function installMainWindowRemoteRequestHeaderPolicy(
  session: Pick<Electron.Session, "webRequest">,
  policy: MainWindowRequestHeaderPolicy,
): () => void {
  session.webRequest.onBeforeRequest?.(
    { urls: REMOTE_REQUEST_URL_PATTERNS },
    (details, callback) => {
      callback(shouldBlockMainWindowExternalRequest(policy, details) ? { cancel: true } : {});
    },
  );
  session.webRequest.onBeforeSendHeaders(
    { urls: REMOTE_REQUEST_URL_PATTERNS },
    (details, callback) => {
      const requestHeaders = stripDesktopOriginHeaders(policy, details);
      callback(requestHeaders === undefined ? {} : { requestHeaders });
    },
  );
  return () => {
    session.webRequest.onBeforeRequest?.(null);
    session.webRequest.onBeforeSendHeaders(null);
  };
}
