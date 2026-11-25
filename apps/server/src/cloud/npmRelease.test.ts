import { expect, it } from "@effect/vitest";
import * as Effect from "effect/Effect";

import * as NpmRelease from "./npmRelease.ts";

it.effect("disables external npm registry lookups and always returns null", () =>
  Effect.gen(function* () {
    const service = yield* NpmRelease.make();
    expect(yield* service.get({ spec: "latest" })).toBeNull();
    expect(yield* service.get({ spec: "nightly" })).toBeNull();
    expect(yield* service.get({ spec: "0.1.5-nightly.20261030.1" })).toBeNull();
  }),
);
