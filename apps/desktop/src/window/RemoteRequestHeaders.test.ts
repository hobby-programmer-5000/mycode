import { assert, describe, it } from "@effect/vitest";

import {
  shouldBlockMainWindowExternalRequest,
  stripDesktopOriginHeaders,
  type MainWindowRequestHeaderPolicy,
} from "./RemoteRequestHeaders.ts";

const packagedPolicy: MainWindowRequestHeaderPolicy = {
  desktopOrigin: "mycode://app",
  mainWebContentsId: 42,
  isDevelopment: false,
};

const devPolicy: MainWindowRequestHeaderPolicy = {
  desktopOrigin: "mycode-dev://app",
  mainWebContentsId: 42,
  devServerOrigin: "http://devbox.internal:5733",
  isDevelopment: true,
};

describe("RemoteRequestHeaders", () => {
  it.each([
    "http://127.0.0.1:3773/.well-known/mycode/environment",
    "http://localhost:3773/api/auth/bootstrap",
    "http://[::1]:3773/api/health",
    "ws://127.0.0.1:3773/ws?ticket=abc",
  ])("preserves Origin and Sec-Fetch-* on production loopback %s requests", (url) => {
    const headers = stripDesktopOriginHeaders(packagedPolicy, {
      url,
      webContentsId: 42,
      frame: { parent: null },
      requestHeaders: {
        Origin: "mycode://app",
        "Sec-Fetch-Site": "cross-site",
        "Sec-Fetch-Mode": "cors",
        "Sec-Fetch-Dest": "empty",
        Authorization: "Bearer secret",
      },
    });

    assert.isUndefined(headers);
    assert.isFalse(shouldBlockMainWindowExternalRequest(packagedPolicy, { url, webContentsId: 42 }));
  });

  it.each([
    "https://remote.example.com/.well-known/mycode/environment",
    "https://telemetry.example.com/api/telemetry",
    "https://auth.example.com/v1/client",
    "https://challenges.cloudflare.com/turnstile/v0/api.js",
    "wss://remote.example.com/ws?ticket=abc",
    "http://192.168.1.20:3773/api/health",
  ])("blocks external outbound request %s and strips desktop origin headers", (url) => {
    assert.isTrue(shouldBlockMainWindowExternalRequest(packagedPolicy, { url, webContentsId: 42 }));
    assert.isTrue(shouldBlockMainWindowExternalRequest(packagedPolicy, { url }));
    assert.isFalse(shouldBlockMainWindowExternalRequest(packagedPolicy, { url, webContentsId: 99 }));

    const headers = stripDesktopOriginHeaders(packagedPolicy, {
      url,
      webContentsId: 42,
      frame: { parent: null },
      requestHeaders: {
        Origin: "mycode://app",
        "Sec-Fetch-Site": "cross-site",
        "Sec-Fetch-Mode": "cors",
        "Sec-Fetch-Dest": "empty",
        "sec-fetch-user": "?1",
        Authorization: "Bearer secret",
        DPoP: "proof-jwt",
      },
    });

    assert.deepEqual(headers, {
      Authorization: "Bearer secret",
      DPoP: "proof-jwt",
    });
  });

  it("strips lowercase origin headers and allows null frames (WebSocket upgrades and workers)", () => {
    const headers = stripDesktopOriginHeaders(packagedPolicy, {
      url: "wss://remote.example.com/ws",
      webContentsId: 42,
      frame: null,
      requestHeaders: {
        origin: "mycode://app",
        "Sec-Fetch-Site": "cross-site",
        Upgrade: "websocket",
      },
    });

    assert.deepEqual(headers, {
      Upgrade: "websocket",
    });
  });

  it("preserves headers for dev loopback and exact devServerOrigin, while stripping other ports and remote hosts", () => {
    for (const url of [
      "http://localhost:3773/api/health",
      "http://app.localhost:3773/api/health",
      "http://127.0.0.1:3773/api/health",
      "http://[::1]:3773/api/health",
      "ws://127.0.0.1:3773/ws",
      "http://devbox.internal:5733/api/health",
      "ws://devbox.internal:5733/ws",
    ]) {
      assert.isFalse(shouldBlockMainWindowExternalRequest(devPolicy, { url, webContentsId: 42 }));
      assert.isUndefined(
        stripDesktopOriginHeaders(devPolicy, {
          url,
          webContentsId: 42,
          frame: { parent: null },
          requestHeaders: {
            Origin: "mycode-dev://app",
            "Sec-Fetch-Site": "cross-site",
          },
        }),
      );
    }

    for (const url of [
      "http://devbox.internal:8080/api/auth/bootstrap",
      "https://remote.example.com/api/auth/bootstrap",
    ]) {
      assert.isTrue(shouldBlockMainWindowExternalRequest(devPolicy, { url, webContentsId: 42 }));
      assert.deepEqual(
        stripDesktopOriginHeaders(devPolicy, {
          url,
          webContentsId: 42,
          frame: { parent: null },
          requestHeaders: {
            Origin: "mycode-dev://app",
            "Sec-Fetch-Site": "cross-site",
            "Content-Type": "application/json",
          },
        }),
        { "Content-Type": "application/json" },
      );
    }
  });

  it("never touches sandboxed iframes (Origin: null), subframes, or other webContents", () => {
    assert.isUndefined(
      stripDesktopOriginHeaders(packagedPolicy, {
        url: "https://remote.example.com/api/test",
        webContentsId: 42,
        frame: { parent: null },
        requestHeaders: {
          Origin: "null",
          "Sec-Fetch-Site": "cross-site",
        },
      }),
    );

    assert.isUndefined(
      stripDesktopOriginHeaders(packagedPolicy, {
        url: "https://remote.example.com/api/test",
        webContentsId: 42,
        frame: { parent: {} },
        requestHeaders: {
          Origin: "mycode://app",
          "Sec-Fetch-Site": "cross-site",
        },
      }),
    );

    assert.isUndefined(
      stripDesktopOriginHeaders(packagedPolicy, {
        url: "https://remote.example.com/api/test",
        webContentsId: 99,
        frame: { parent: null },
        requestHeaders: {
          Origin: "mycode://app",
          "Sec-Fetch-Site": "cross-site",
        },
      }),
    );

    assert.isUndefined(
      stripDesktopOriginHeaders(packagedPolicy, {
        url: "https://remote.example.com/api/test",
        webContentsId: undefined,
        frame: null,
        requestHeaders: {
          Origin: "mycode://app",
          "Sec-Fetch-Site": "cross-site",
        },
      }),
    );

    assert.isUndefined(
      stripDesktopOriginHeaders(packagedPolicy, {
        url: "https://remote.example.com/api/test",
        webContentsId: 42,
        frame: { parent: null },
        requestHeaders: {
          "Sec-Fetch-Site": "cross-site",
        },
      }),
    );
  });
});
