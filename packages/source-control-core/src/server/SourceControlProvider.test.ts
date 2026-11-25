import { assert, it } from "@effect/vitest";

import {
  repositoryPathFromRemoteUrl,
  transportSafeSourceControlErrorValue,
} from "./SourceControlProvider.ts";

it("removes URL credentials, query parameters, and fragments from error transport values", () => {
  assert.strictEqual(
    transportSafeSourceControlErrorValue(
      "https://user:secret@example.test/org/repo/pull/42?token=secret#discussion",
    ),
    "https://example.test/org/repo/pull/42",
  );
});

it.each([
  ["git@github.com:hobby-programmer-5000/mycode.git", "hobby-programmer-5000/mycode"],
  ["github.com:hobby-programmer-5000/mycode.git", "hobby-programmer-5000/mycode"],
  ["gitlab.com:hobby-programmer-5000/platform/mycode", "hobby-programmer-5000/platform/mycode"],
  ["https://github.com/hobby-programmer-5000/mycode.git", "hobby-programmer-5000/mycode"],
  ["https://github.com/hobby-programmer-5000/mycode.git?view=compact", "hobby-programmer-5000/mycode"],
  ["https://github.com/hobby-programmer-5000/mycode.git#section", "hobby-programmer-5000/mycode"],
  ["C:/repos/mycode", null],
])("reads the repository path of %s", (remoteUrl, expected) => {
  assert.strictEqual(repositoryPathFromRemoteUrl(remoteUrl), expected);
});
