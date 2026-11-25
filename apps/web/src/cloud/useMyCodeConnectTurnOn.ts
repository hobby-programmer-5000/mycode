import {
  AuthRelayReadScope,
  AuthRelayWriteScope,
  type EnvironmentId,
  managedCapabilityDenial,
} from "@mycode/contracts";

import { useManagedPolicy } from "../state/managedPolicy";
import { useEnvironmentCanConfigureRelay, useEnvironmentScope } from "../state/session";
import { useCloudLinkState } from "./cloudLinkState";
import { hasCloudPublicConfig } from "./publicConfig";
import { resolveT3ConnectTurnOn } from "./myCodeConnectState";

/** Whether this client can turn on MyCode Connect for `environmentId`, and if not, why. */
export function useMyCodeConnectTurnOn(environmentId: EnvironmentId | null) {
  const canReadRelay = useEnvironmentScope(environmentId, AuthRelayReadScope);
  const canWriteRelay = useEnvironmentScope(environmentId, AuthRelayWriteScope);
  const canConfigureRelay = useEnvironmentCanConfigureRelay(environmentId);
  const connectDenial = managedCapabilityDenial(useManagedPolicy(environmentId), "connect");
  const linkState = useCloudLinkState(environmentId).data;
  return resolveT3ConnectTurnOn({
    // Unknown while loading and on older servers, which keep today's switch in Settings.
    linkableHere: linkState?.linkableHere ?? true,
    cloudConfigured: hasCloudPublicConfig(),
    canReadRelay,
    canWriteRelay: canWriteRelay && canConfigureRelay,
    connectDenial,
  });
}
