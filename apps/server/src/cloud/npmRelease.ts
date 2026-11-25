import {
  ServerNpmReleaseError,
  ServerNpmReleaseInput,
  type ServerNpmRelease,
} from "@mycode/contracts";
import * as Context from "effect/Context";
import * as Effect from "effect/Effect";
import * as Layer from "effect/Layer";

export class NpmRelease extends Context.Service<
  NpmRelease,
  {
    readonly get: (
      input: ServerNpmReleaseInput,
    ) => Effect.Effect<ServerNpmRelease | null, ServerNpmReleaseError>;
  }
>()("mycode/cloud/npmRelease") {}

/** External npm registry phone-home checks are disabled. */
export const make = Effect.fn("NpmRelease.make")(() =>
  Effect.succeed(
    NpmRelease.of({
      get: Effect.fn("NpmRelease.get")(() => Effect.succeed(null)),
    }),
  ),
);

export const layer = Layer.effect(NpmRelease, make());
