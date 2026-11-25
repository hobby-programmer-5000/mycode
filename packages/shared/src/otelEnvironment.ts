import * as ConfigProvider from "effect/ConfigProvider";
import * as Data from "effect/Data";
import * as Effect from "effect/Effect";

import type { OtlpEndpointSource } from "@mycode/contracts";

import { type OtlpProtocol, type SignalExport } from "./observability.ts";

export type OtelSignal = Data.TaggedEnum<{
  Unset: {};
  Off: {};
  Export: {
    readonly url: string;
    readonly variable: string;
    readonly protocol: OtlpProtocol;
    readonly headers: Readonly<Record<string, string>> | undefined;
  };
}>;
export const OtelSignal = Data.taggedEnum<OtelSignal>();

export interface OtelEnvironment {
  readonly disabled: boolean;
  readonly warnings: ReadonlyArray<string>;
  readonly resourceAttributes: Readonly<Record<string, string>>;
  readonly traces: OtelSignal;
  readonly metrics: OtelSignal;
  readonly logs: OtelSignal;
}

const RESOURCE_ATTRIBUTES = "OTEL_RESOURCE_ATTRIBUTES";

export const load: Effect.Effect<OtelEnvironment> = Effect.succeed({
  disabled: true,
  warnings: [],
  resourceAttributes: {},
  traces: OtelSignal.Off(),
  metrics: OtelSignal.Off(),
  logs: OtelSignal.Off(),
});

export const isRelayTracingEnabled = (
  _env: Readonly<Record<string, string | undefined>>,
): boolean => false;

export type SignalName = "traces" | "metrics" | "logs";

export interface SignalEndpoint {
  readonly url: string;
  readonly export: SignalExport;
  readonly source: OtlpEndpointSource;
  /** The environment variable that set the endpoint, when `source` is `environment`. */
  readonly variable?: string;
}

export const resolveSignalEndpoint = (
  _otel: OtelEnvironment,
  _signal: SignalName,
  _t3: { readonly url: string | undefined; readonly export: SignalExport },
  _fallback: {
    readonly bootstrapUrl: string | undefined;
    readonly settingsUrl: string | undefined;
  },
): SignalEndpoint | undefined => undefined;

/** Give OTLP exporters the accepted resource attributes so their own decoding cannot fail. */
export const layerResourceAttributes = (attributes: Readonly<Record<string, string>>) =>
  ConfigProvider.layerAdd(
    ConfigProvider.fromEnv({
      env: {
        [RESOURCE_ATTRIBUTES]: Object.entries(attributes)
          .map(([key, value]) => `${encodeURIComponent(key)}=${encodeURIComponent(value)}`)
          .join(","),
      },
      preserveEmptyStrings: true,
    }),
    { asPrimary: true },
  );

export const none: OtelEnvironment = {
  disabled: true,
  warnings: [],
  resourceAttributes: {},
  traces: OtelSignal.Off(),
  metrics: OtelSignal.Off(),
  logs: OtelSignal.Off(),
};
