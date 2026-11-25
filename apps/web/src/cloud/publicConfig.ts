import { relayClerkTokenOptions } from "@mycode/shared/relayAuth";
import * as Schema from "effect/Schema";

export class CloudPublicConfigMissingError extends Schema.TaggedError<CloudPublicConfigMissingError>()(
  "CloudPublicConfigMissingError",
  {
    key: Schema.Literal("MYCODE_CLERK_JWT_TEMPLATE"),
  },
) {
  override get message(): string {
    return `${this.key} is not configured.`;
  }
}

export interface CloudPublicConfig {
  readonly clerkPublishableKey: string | null;
  readonly clerkJwtTemplate: string | null;
  readonly relayUrl: string | null;
  readonly relayTracing: {
    readonly tracesUrl: string | null;
    readonly tracesDataset: string | null;
    readonly tracesToken: string | null;
  };
}

export function trimNonEmpty(value: string | undefined): string | null {
  return value?.trim() || null;
}

export function resolveCloudPublicConfig(): CloudPublicConfig {
  return {
    clerkPublishableKey: null,
    clerkJwtTemplate: null,
    relayUrl: null,
    relayTracing: {
      tracesUrl: null,
      tracesDataset: null,
      tracesToken: null,
    },
  };
}

export function setServerRelayTelemetry(_allowed: boolean | null): void {}

export function relayTelemetryEnabled(): boolean | null {
  return false;
}

export function resolveRelayTracingConfig(): null {
  return null;
}

export function hasCloudPublicConfig(): boolean {
  return false;
}

export function resolveRelayClerkTokenOptions() {
  const { clerkJwtTemplate } = resolveCloudPublicConfig();
  if (!clerkJwtTemplate) {
    throw new CloudPublicConfigMissingError({ key: "MYCODE_CLERK_JWT_TEMPLATE" });
  }
  return relayClerkTokenOptions(clerkJwtTemplate);
}
