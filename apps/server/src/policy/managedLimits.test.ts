import type { ManagedPolicyDocument } from "@mycode/contracts";
import { assert, it } from "@effect/vitest";
import * as Effect from "effect/Effect";

import * as EnvironmentAuthPolicy from "../auth/EnvironmentAuthPolicy.ts";
import * as ManagedLimits from "./managedLimits.ts";

const policy = (limits?: ManagedPolicyDocument["limits"]) => ({
  document: {
    schemaVersion: 1 as const,
    organization: "Example Corp",
    ...(limits === undefined ? {} : { limits }),
  },
});

const desktopAndLocal = policy({ clients: ["desktop", "local-browser"] });

it("allows auth policies, remote clients, and connect without corporate policy locks", () => {
  assert.isTrue(ManagedLimits.allowsAuthPolicy(desktopAndLocal, "unsafe-no-auth"));
  assert.isTrue(ManagedLimits.allowsAuthPolicy(desktopAndLocal, "remote-reachable"));
  assert.isTrue(ManagedLimits.allowsRemoteClients(desktopAndLocal));
  assert.isTrue(ManagedLimits.allowsConnect(desktopAndLocal));
  assert.equal(
    EnvironmentAuthPolicy.resolveServerAuthPolicy({ mode: "web", host: "0.0.0.0" }),
    "unsafe-no-auth",
  );
});

it.effect("defaults bind host to 127.0.0.1 and honors explicitly requested remote hosts", () =>
  Effect.gen(function* () {
    assert.equal(
      yield* ManagedLimits.checkServerReachability(desktopAndLocal, {
        authPolicy: "unsafe-no-auth",
        host: undefined,
        tailscaleServeEnabled: false,
      }),
      "127.0.0.1",
    );
    assert.equal(
      yield* ManagedLimits.checkServerReachability(desktopAndLocal, {
        authPolicy: "unsafe-no-auth",
        host: "0.0.0.0",
        tailscaleServeEnabled: true,
      }),
      "0.0.0.0",
    );
  }),
);
