import type { ManagedPolicySummary } from "@mycode/contracts";
import {
  nodeProcessRuntimeAttributes,
  processServiceInstanceId,
} from "@mycode/shared/observability";
import * as RelayTracing from "@mycode/shared/relayTracing";

import { resolveRelayClientTracingConfig } from "./publicConfig.ts";

const relayClientTracingConfig = resolveRelayClientTracingConfig();

/**
 * `false` once the environment or a managed policy opted out. Until the desktop has read the
 * login shell, a shell-profile opt-out is still unknown, so tracing stays off.
 */
export const allowedFor = (
  _env: Readonly<Record<string, string | undefined>>,
  _policy: ManagedPolicySummary | undefined,
): boolean => false;

/** Relay telemetry export is permanently disabled. */
export const relayLayer = (
  resource: Omit<RelayTracing.RelayClientTracingResource, "serviceInstanceId">,
  _tracingConfig: RelayTracing.RelayClientTracingConfig | null = relayClientTracingConfig,
) =>
  RelayTracing.layer(
    null,
    { ...resource, serviceInstanceId: processServiceInstanceId() },
    { isEnabled: () => false },
  );

export const layerHeadlessRelayClient = relayLayer({
  serviceName: "mycode-server",
  runtime: "node",
  attributes: nodeProcessRuntimeAttributes(),
  client: "headless-cli",
});

export const layerServerRelayBroker = relayLayer({
  serviceName: "mycode-server",
  runtime: "node",
  attributes: nodeProcessRuntimeAttributes(),
  client: "environment-server",
  component: "relay-broker",
});
