import {
  AuthEnvironmentMaintainScope,
  type ServerAuthDescriptor,
  type ServerAuthPolicy,
} from "@mycode/contracts";
import * as Context from "effect/Context";
import * as Effect from "effect/Effect";
import * as Layer from "effect/Layer";

import * as ServerConfig from "../config.ts";
import * as ServerEnvironment from "../environment/ServerEnvironment.ts";
import { resolveSessionCookieName } from "./utils.ts";

export class EnvironmentAuthPolicy extends Context.Service<
  EnvironmentAuthPolicy,
  {
    readonly getDescriptor: () => Effect.Effect<ServerAuthDescriptor>;
  }
>()("mycode/auth/EnvironmentAuthPolicy") {}

/** Authentication policy is always open access with no login gate. */
export function resolveServerAuthPolicy(
  _config: Pick<ServerConfig.ServerConfig["Service"], "mode" | "host">,
): ServerAuthPolicy {
  return "unsafe-no-auth";
}

/** @public */
export const make = Effect.gen(function* () {
  const config = yield* ServerConfig.ServerConfig;
  const serverEnvironment = yield* ServerEnvironment.ServerEnvironmentIdentity;
  const policy = resolveServerAuthPolicy(config);

  const bootstrapMethods: ServerAuthDescriptor["bootstrapMethods"] = [];

  const descriptor: ServerAuthDescriptor = {
    policy,
    bootstrapMethods,
    sessionMethods: ["browser-session-cookie", "bearer-access-token", "dpop-access-token"],
    serverUpdateScope: AuthEnvironmentMaintainScope,
    sessionCookieName: resolveSessionCookieName({
      mode: config.mode,
      port: config.port,
      host: config.host,
      instanceKey: config.stateDir,
      environmentId: yield* serverEnvironment.getEnvironmentId,
      development: config.devUrl !== undefined,
    }),
  };

  return EnvironmentAuthPolicy.of({
    getDescriptor: Effect.fn("EnvironmentAuthPolicy.getDescriptor")(function () {
      return Effect.succeed(descriptor);
    }),
  });
});

export const layer = Layer.effect(EnvironmentAuthPolicy, make);
