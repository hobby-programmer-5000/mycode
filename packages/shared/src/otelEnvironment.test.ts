import { assert, describe, it } from "@effect/vitest";
import * as ConfigProvider from "effect/ConfigProvider";
import * as Effect from "effect/Effect";
import * as Layer from "effect/Layer";
import * as OtlpResource from "effect/observability/OtlpResource";

import * as OtelEnvironment from "./otelEnvironment.ts";

const load = (env: Record<string, string>) =>
  OtelEnvironment.load.pipe(Effect.provide(ConfigProvider.layer(ConfigProvider.fromEnv({ env }))));

describe("OtelEnvironment", () => {
  it.effect.each([
    { name: "nothing set", env: {} },
    { name: "spec endpoint set", env: { OTEL_EXPORTER_OTLP_ENDPOINT: "https://collector:4318" } },
    {
      name: "explicit enable override",
      env: {
        MYCODE_OTEL_SDK_DISABLED: "false",
        OTEL_SDK_DISABLED: "false",
        OTEL_EXPORTER_OTLP_TRACES_ENDPOINT: "https://tempo:4318/v1/traces",
      },
    },
  ])("always disables telemetry exports ($name)", ({ env }) =>
    Effect.gen(function* () {
      const resolved = yield* load(env);
      assert.isTrue(resolved.disabled);
      assert.deepStrictEqual(resolved.warnings, []);
      assert.deepStrictEqual(resolved.resourceAttributes, {});
      assert.deepStrictEqual(resolved.traces, OtelEnvironment.OtelSignal.Off());
      assert.deepStrictEqual(resolved.metrics, OtelEnvironment.OtelSignal.Off());
      assert.deepStrictEqual(resolved.logs, OtelEnvironment.OtelSignal.Off());
    }),
  );

  it("resolveSignalEndpoint always returns undefined", () => {
    const t3Export = {
      protocol: "http/json",
      headers: { "x-key": "t3" },
      exportIntervalMs: 5_000,
    } as const;
    assert.isUndefined(
      OtelEnvironment.resolveSignalEndpoint(
        OtelEnvironment.none,
        "logs",
        { url: "http://t3:4318/v1/logs", export: t3Export },
        { bootstrapUrl: "http://desktop:4318/v1/logs", settingsUrl: "http://settings:4318/v1/logs" },
      ),
    );
  });

  it("isRelayTracingEnabled always returns false", () => {
    assert.isFalse(OtelEnvironment.isRelayTracingEnabled({}));
    assert.isFalse(
      OtelEnvironment.isRelayTracingEnabled({
        OTEL_SDK_DISABLED: "false",
        MYCODE_OTEL_SDK_DISABLED: "false",
        MYCODE_TELEMETRY_ENABLED: "true",
      }),
    );
  });

  describe("layerResourceAttributes", () => {
    it.effect("lets OtlpResource.fromConfig read encoded attributes cleanly", () =>
      Effect.gen(function* () {
        const layerEnv = ConfigProvider.layer(ConfigProvider.fromEnv({ env: {} }));
        const resource = yield* OtlpResource.fromConfig({ serviceName: "mycode" }).pipe(
          Effect.provide(
            Layer.provide(OtelEnvironment.layerResourceAttributes({ "a,b": "x=y" }), layerEnv),
          ),
        );
        assert.deepStrictEqual(
          resource.attributes
            .map((attribute) => attribute.key)
            .filter((key) => !key.startsWith("telemetry.sdk.")),
          ["a,b", "service.name"],
        );
      }),
    );
  });
});
