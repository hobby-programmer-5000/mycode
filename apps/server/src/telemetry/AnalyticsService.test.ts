import * as NodeServices from "@effect/platform-node/NodeServices";
import { assert, it } from "@effect/vitest";
import * as Effect from "effect/Effect";
import * as Layer from "effect/Layer";
import * as HttpClient from "effect/http/HttpClient";
import * as HttpClientResponse from "effect/http/HttpClientResponse";

import * as ServerConfig from "../config.ts";
import { getServerOwners, getTelemetryIdentity } from "./Identify.ts";
import * as AnalyticsService from "./AnalyticsService.ts";

it.effect("never sends outbound analytics requests and returns no-op telemetry identity", () =>
  Effect.gen(function* () {
    let outboundRequests = 0;
    const client = HttpClient.make((request) => {
      outboundRequests += 1;
      return Effect.succeed(HttpClientResponse.fromWeb(request, new Response(null, { status: 204 })));
    });

    const result = yield* Effect.gen(function* () {
      const analytics = yield* AnalyticsService.AnalyticsService;
      const recorded = yield* analytics.record("server.started", { mode: "web" });
      yield* analytics.recordOwners([{ source: "connect", ownerId: "abc" }], "server");
      yield* analytics.flush;
      const identity = yield* getTelemetryIdentity;
      const owners = yield* getServerOwners;
      return { recorded, identity, owners };
    }).pipe(
      Effect.provide(
        AnalyticsService.layer.pipe(
          Layer.provideMerge(
            ServerConfig.layerTest(process.cwd(), { prefix: "mycode-telemetry-off-" }),
          ),
          Layer.provideMerge(Layer.succeed(HttpClient.HttpClient, client)),
          Layer.provideMerge(NodeServices.layer),
        ),
      ),
    );

    assert.isFalse(result.recorded);
    assert.deepEqual(result.identity, { distinctId: null, installId: null });
    assert.deepEqual(result.owners, []);
    assert.equal(outboundRequests, 0);
  }),
);
