import type { DatabaseMigrationProgress as Progress } from "@mycode/contracts";
import * as Context from "effect/Context";
import * as Layer from "effect/Layer";
import * as Ref from "effect/Ref";
import * as Effect from "effect/Effect";

export class DatabaseMigrationProgress extends Context.Service<
  DatabaseMigrationProgress,
  { readonly current: Ref.Ref<Progress | null> }
>()("mycode-server/persistence/DatabaseMigrationProgress") {}

export const layer = Layer.effect(
  DatabaseMigrationProgress,
  Effect.map(Ref.make<Progress | null>(null), (current) => ({ current })),
);
