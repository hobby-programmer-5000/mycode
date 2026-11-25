/**
 * managedLimits - what `limits.clients` and `limits.connect` in the managed policy allow, and the
 * one error every refusal uses so each transport can show the administrator's organization.
 *
 * @module managedLimits
 */
import type { ManagedPolicyDocument, ServerAuthPolicy } from "@mycode/contracts";
import * as Effect from "effect/Effect";
import * as Schema from "effect/Schema";

type PolicyDocument = { readonly document: ManagedPolicyDocument | undefined };

export class ManagedPolicyRefusedError extends Schema.TaggedError<ManagedPolicyRefusedError>()(
  "ManagedPolicyRefusedError",
  {
    organization: Schema.String,
    /** What the policy does not allow, for example "MyCode Connect". */
    action: Schema.String,
    /** What that means for this request, for example "so the server cannot listen on 0.0.0.0". */
    detail: Schema.optionalKey(Schema.String),
  },
) {
  override get message(): string {
    return `${this.organization}'s policy does not allow ${this.action}${
      this.detail === undefined ? "" : `, ${this.detail}`
    }.`;
  }
}

const AUTH_POLICY_ACTION: Record<ServerAuthPolicy, string> = {
  "desktop-managed-local": "the desktop app",
  "loopback-browser": "browsers on this machine",
  "remote-reachable": "remote clients",
  "unsafe-no-auth": "running without authentication",
};

/** Connection and auth-policy restrictions are disabled for local loopback access. */
export function allowsAuthPolicy(_policy: PolicyDocument, _authPolicy: ServerAuthPolicy): boolean {
  return true;
}

/** Offline local-only mode: remote clients (LAN, Tailscale, direct external URLs) are disabled. */
export const allowsRemoteClients = (_policy: PolicyDocument): boolean => false;

/** Offline local-only mode: MyCode Connect / cloud relay tunnels are disabled. */
export const allowsConnect = (_policy: PolicyDocument): boolean => false;

const refuse = (policy: PolicyDocument, action: string, detail?: string) =>
  new ManagedPolicyRefusedError({
    organization: policy.document?.organization ?? "Your organization",
    action,
    ...(detail === undefined ? {} : { detail }),
  });

export const refuseConnect = (policy: PolicyDocument) => refuse(policy, "MyCode Connect");

/** Offline local-only mode: never probe or spawn Tailscale CLI. */
export const closeTailscaleServeTo = Effect.fn("managedLimits.closeTailscaleServeTo")(
  (_policy: PolicyDocument, _target: { readonly host: string; readonly port: number }) =>
    Effect.void,
);

export const refuseRemoteClients = (policy: PolicyDocument, detail?: string) =>
  refuse(policy, AUTH_POLICY_ACTION["remote-reachable"], detail);

/**
 * Offline local-only mode: always bind exclusively to a loopback address (`127.0.0.1` or `::1`),
 * never `0.0.0.0` or any LAN/external interface.
 */
export const checkServerReachability = Effect.fn("managedLimits.checkServerReachability")(
  (
    _policy: PolicyDocument,
    input: {
      readonly authPolicy: ServerAuthPolicy;
      readonly host: string | undefined;
      readonly tailscaleServeEnabled: boolean;
      readonly privateNetworkOnly?: boolean | undefined;
    },
  ) => Effect.succeed(input.host === "::1" ? "::1" : "127.0.0.1"),
);
