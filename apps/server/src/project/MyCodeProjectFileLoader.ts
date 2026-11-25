import * as Context from "effect/Context";
import * as Effect from "effect/Effect";
import * as FileSystem from "effect/FileSystem";
import * as Layer from "effect/Layer";
import * as Option from "effect/Option";
import * as Path from "effect/Path";
import * as Schema from "effect/Schema";

import { T3_PROJECT_FILE_NAME, type MyCodeProjectFile } from "@mycode/contracts";
import { MyCodeProjectFileFromJson } from "@mycode/shared/myCodeProjectFile";

const decodeMyCodeProjectFileJson = Schema.decodeEffect(MyCodeProjectFileFromJson);

export class MyCodeProjectFileLoadError extends Schema.TaggedError<MyCodeProjectFileLoadError>()(
  "MyCodeProjectFileLoadError",
  {
    operation: Schema.Literals(["read", "decode"]),
    workspaceRoot: Schema.String,
    filePath: Schema.String,
    cause: Schema.Defect(),
  },
) {
  override get message(): string {
    return `Failed to ${this.operation} ${T3_PROJECT_FILE_NAME} at ${this.filePath}.`;
  }
}

export class MyCodeProjectFileLoader extends Context.Service<
  MyCodeProjectFileLoader,
  {
    /** Never fails; missing/unreadable/invalid files return none, with warnings for invalid files. */
    readonly load: (workspaceRoot: string) => Effect.Effect<Option.Option<MyCodeProjectFile>>;
  }
>()("mycode/project/MyCodeProjectFileLoader") {}

const logMyCodeProjectFileLoadError = (error: MyCodeProjectFileLoadError) =>
  Effect.logWarning(error).pipe(
    Effect.annotateLogs({
      operation: error.operation,
      workspaceRoot: error.workspaceRoot,
      filePath: error.filePath,
      errorTag: error._tag,
    }),
  );

/** @public */
export const make = Effect.gen(function* () {
  const fileSystem = yield* FileSystem.FileSystem;
  const path = yield* Path.Path;

  const load: MyCodeProjectFileLoader["Service"]["load"] = Effect.fn("MyCodeProjectFileLoader.load")(
    function* (workspaceRoot) {
      const filePath = path.join(workspaceRoot, T3_PROJECT_FILE_NAME);
      const raw = yield* fileSystem.readFileString(filePath).pipe(
        Effect.asSome,
        Effect.catchTags({
          PlatformError: (error) =>
            error.reason._tag === "NotFound"
              ? Effect.succeed(Option.none<string>())
              : logMyCodeProjectFileLoadError(
                  new MyCodeProjectFileLoadError({
                    operation: "read",
                    workspaceRoot,
                    filePath,
                    cause: error,
                  }),
                ).pipe(Effect.as(Option.none<string>())),
        }),
      );
      if (Option.isNone(raw)) {
        return Option.none<MyCodeProjectFile>();
      }
      return yield* decodeMyCodeProjectFileJson(raw.value).pipe(
        Effect.asSome,
        Effect.catchTags({
          SchemaError: (error) =>
            logMyCodeProjectFileLoadError(
              new MyCodeProjectFileLoadError({
                operation: "decode",
                workspaceRoot,
                filePath,
                cause: error,
              }),
            ).pipe(Effect.as(Option.none<MyCodeProjectFile>())),
        }),
      );
    },
  );

  return MyCodeProjectFileLoader.of({ load });
});

export const layer = Layer.effect(MyCodeProjectFileLoader, make);
