/**
 * DesktopUpdatePolicy - what the administrator's managed policy allows the desktop updater to do.
 * The desktop reads the policy file itself, before any backend runs, and fails closed: a policy it
 * cannot read turns updates off.
 *
 * @module DesktopUpdatePolicy
 */
import type { ManagedUpdatesMode } from "@mycode/contracts";
import * as Context from "effect/Context";
import * as Effect from "effect/Effect";
import * as Layer from "effect/Layer";

export interface UpdatePolicy {
  readonly mode: ManagedUpdatesMode;
  readonly feedUrl: string | undefined;
  readonly allowNightly: boolean;
  /** Why the updater is restricted, shown in place of update actions; `null` when unmanaged. */
  readonly message: string | null;
}

export class DesktopUpdatePolicy extends Context.Service<DesktopUpdatePolicy, UpdatePolicy>()(
  "@mycode/desktop/updates/DesktopUpdatePolicy",
) {}

export const unmanagedUpdatePolicy: UpdatePolicy = {
  mode: "auto",
  feedUrl: undefined,
  allowNightly: true,
  message: null,
};

const offlineDisabledUpdatePolicy: UpdatePolicy = {
  mode: "off",
  feedUrl: undefined,
  allowNightly: false,
  message: "Automatic updates are disabled in offline local-only mode.",
};

const make = Effect.succeed<UpdatePolicy>(offlineDisabledUpdatePolicy);

export const layer = Layer.effect(DesktopUpdatePolicy, make);

export const layerTest = (policy: UpdatePolicy = unmanagedUpdatePolicy) =>
  Layer.succeed(DesktopUpdatePolicy, policy);
