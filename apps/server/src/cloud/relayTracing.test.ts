import { assert, it, vi } from "@effect/vitest";
import type { ManagedPolicyDocument } from "@mycode/contracts";
import * as HostProcess from "@mycode/shared/HostProcess";
import * as RelayTracing from "@mycode/shared/relayTracing";
import * as Effect from "effect/Effect";
import * as Layer from "effect/Layer";
import * as Option from "effect/Option";
import { FetchHttpClient } from "effect/http";

import * as ManagedPolicy from "../policy/ManagedPolicy.ts";
import { allowedFor, relayLayer } from "./relayTracing.ts";

const tracesTo = (policy?: ManagedPolicyDocument) =>
  Effect.service(RelayTracing.RelayClientTracer).pipe(
    Effect.provide(
      relayLayer(
        { serviceName: "mycode-server", runtime: "node", client: "environment-server" },
        { tracesUrl: "https://traces.example.com", tracesDataset: "relay", tracesToken: "token" },
      ).pipe(Layer.provide(ManagedPolicy.layerTest(policy)), Layer.provide(FetchHttpClient.layer)),
    ),
    Effect.map(Option.isSome),
  );

it.effect("relay tracing is permanently disabled regardless of policy", () =>
  Effect.gen(function* () {
    assert.isFalse(yield* tracesTo());
    assert.isFalse(
      yield* tracesTo({
        schemaVersion: 1,
        organization: "Example Corp",
        limits: { telemetry: false },
      }),
    );
  }),
);

it.effect("relay tracing never exports spans over HTTP", () => {
  const fetchFn = vi.fn<typeof fetch>(async () => new Response(null, { status: 202 }));
  const environment: NodeJS.ProcessEnv = { MYCODE_DESKTOP_SHELL_DISCOVERY_PENDING: "1" };
  return Effect.void.pipe(
    Effect.withSpan("relay.request"),
    RelayTracing.withRelayClientTracing,
    Effect.andThen(
      Effect.sync(() => {
        assert.isFalse(allowedFor(environment, undefined));
        delete environment.MYCODE_DESKTOP_SHELL_DISCOVERY_PENDING;
        assert.isFalse(allowedFor(environment, undefined));
      }),
    ),
    Layer.effectDiscard,
    Layer.provide(
      relayLayer(
        { serviceName: "mycode-server", runtime: "node", client: "environment-server" },
        { tracesUrl: "https://traces.example.com", tracesDataset: "relay", tracesToken: "token" },
      ),
    ),
    Layer.provide(ManagedPolicy.layerTest()),
    Layer.provide(Layer.succeed(HostProcess.Environment, environment)),
    Layer.provide(FetchHttpClient.layer),
    Layer.provide(Layer.succeed(FetchHttpClient.Fetch, fetchFn)),
    Layer.build,
    Effect.scoped,
    Effect.andThen(Effect.sync(() => assert.strictEqual(fetchFn.mock.calls.length, 0))),
  );
});

it("always reports relay tracing disabled to clients", () => {
  assert.isFalse(allowedFor({}, undefined));
  assert.isFalse(allowedFor({ MYCODE_TELEMETRY_ENABLED: "false" }, undefined));
  assert.isFalse(
    allowedFor(
      {},
      { organization: "Example Corp", lockedSettings: [], limits: { telemetry: false } },
    ),
  );
});
