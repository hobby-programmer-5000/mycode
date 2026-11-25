import * as NodeCrypto from "@effect/platform-node/NodeCrypto";
import * as NodeServices from "@effect/platform-node/NodeServices";
import { assert, it } from "@effect/vitest";
import * as Effect from "effect/Effect";
import * as FileSystem from "effect/FileSystem";
import * as Layer from "effect/Layer";
import * as Path from "effect/Path";

import * as HostProcess from "@mycode/shared/HostProcess";
import * as ServerSecretStore from "../auth/ServerSecretStore.ts";
import { CLOUD_LINKED_USER_ID } from "../cloud/config.ts";
import * as ServerConfig from "../config.ts";
import * as Identify from "./Identify.ts";

const layerIdentity = (prefix: string) =>
  Layer.provideMerge(ServerSecretStore.layer, ServerConfig.layerTest(process.cwd(), { prefix }));

const writeHomeFile = Effect.fn("test.writeHomeFile")(function* (
  homeDirectory: string,
  relativePath: string,
  contents: string,
) {
  const fileSystem = yield* FileSystem.FileSystem;
  const path = yield* Path.Path;
  const filePath = path.join(homeDirectory, relativePath);
  yield* fileSystem.makeDirectory(path.dirname(filePath), { recursive: true });
  yield* fileSystem.writeFileString(filePath, contents);
  return filePath;
});

it.layer(Layer.mergeAll(NodeServices.layer, NodeCrypto.layer))("telemetry identity", (it) => {
  it.effect("never reads or persists an anonymous telemetry identity", () =>
    Effect.gen(function* () {
      const config = yield* ServerConfig.ServerConfig;
      const fileSystem = yield* FileSystem.FileSystem;
      const path = yield* Path.Path;
      const homeDirectory = path.join(config.baseDir, "home");
      yield* writeHomeFile(homeDirectory, ".codex/auth.json", `{"tokens":{"account_id":"ws-1"}}`);
      yield* writeHomeFile(homeDirectory, ".claude.json", `{"userID":"claude-machine"}`);

      assert.deepEqual(
        yield* Identify.getTelemetryIdentity.pipe(
          Effect.provideService(HostProcess.HomeDirectory, homeDirectory),
        ),
        { distinctId: null, installId: null },
      );
      assert.isFalse(yield* fileSystem.exists(config.anonymousIdPath));
    }).pipe(Effect.provide(layerIdentity("t3-telemetry-identify-install-"))),
  );

  it.effect("returns empty owners even when a Connect account secret exists", () =>
    Effect.gen(function* () {
      const secrets = yield* ServerSecretStore.ServerSecretStore;
      assert.deepEqual(yield* Identify.getServerOwners, []);

      yield* secrets.set(CLOUD_LINKED_USER_ID, new TextEncoder().encode("user_connect"));

      assert.deepEqual(yield* Identify.getServerOwners, []);
    }).pipe(Effect.provide(layerIdentity("t3-telemetry-identify-owners-"))),
  );
});
