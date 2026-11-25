import * as Schema from "effect/Schema";
import { describe, expect, it } from "vite-plus/test";

import { parseMyCodeProjectFile, MyCodeProjectFileFromJson } from "./myCodeProjectFile.ts";

const decodeJson = Schema.decodeUnknownSync(MyCodeProjectFileFromJson);

describe("MyCodeProjectFileFromJson", () => {
  it("decodes lenient JSONC with comments and trailing commas", () => {
    const decoded = decodeJson(`{
      // team scripts
      "iconPath": "assets/logo.svg",
      "scripts": [
        { "name": "Dev", "command": "pnpm dev", "stopOnSettle": true, },
      ],
    }`);

    expect(decoded.iconPath).toBe("assets/logo.svg");
    expect(decoded.scripts?.[0]).toEqual({ name: "Dev", command: "pnpm dev", stopOnSettle: true });
  });
});

describe("parseMyCodeProjectFile", () => {
  it("reads project defaults from JSONC with a URL in a block comment", () => {
    expect(
      parseMyCodeProjectFile(`{
        /* See http://127.0.0.1:3773/schema/t3.json */
        "defaultThreadEnvMode": "worktree"
      }`),
    ).toEqual({ defaultThreadEnvMode: "worktree" });
  });
});
