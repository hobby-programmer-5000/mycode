import * as AzureDevOps from "@mycode/source-control-azure-devops/shared/definition";
import * as Bitbucket from "@mycode/source-control-bitbucket/shared/definition";
import { makeSourceControlDefinitionRegistry } from "@mycode/source-control-core/shared/definition";
import * as CursorOrigin from "@mycode/source-control-cursor-origin/shared/definition";
import * as Forgejo from "@mycode/source-control-forgejo/shared/definition";
import * as GitCafe from "@mycode/source-control-gitcafe/shared/definition";
import * as GitHub from "@mycode/source-control-github/shared/definition";
import * as GitLab from "@mycode/source-control-gitlab/shared/definition";
import * as TanStackSource from "@mycode/source-control-tanstack-source/shared/definition";

export { canonicalizePullRequestUrl as canonicalizeGitCafePullRequestUrl } from "@mycode/source-control-gitcafe/shared/definition";

/**
 * GitHub comes first as the client default for unknown hosts. GitCafe, TanStack Source and Origin
 * claim URLs first: GitHub and Forgejo accept the same routes on any hostname. TanStack Source is a
 * private beta: every reader knows its URLs, and only a server with its flag on offers the forge.
 */
export const builtInSourceControlDefinitions = makeSourceControlDefinitionRegistry(
  [
    GitHub.definition,
    CursorOrigin.definition,
    GitLab.definition,
    GitCafe.definition,
    TanStackSource.definition,
    Forgejo.definition,
    Bitbucket.definition,
    AzureDevOps.definition,
  ],
  {
    claimChangeRequestUrlsFirst: [
      GitCafe.definition.kind,
      TanStackSource.definition.kind,
      CursorOrigin.definition.kind,
      AzureDevOps.definition.kind,
    ],
    recognizeRemotesIn: [
      GitCafe.definition.kind,
      TanStackSource.definition.kind,
      CursorOrigin.definition.kind,
      Forgejo.definition.kind,
      GitHub.definition.kind,
      GitLab.definition.kind,
      AzureDevOps.definition.kind,
      Bitbucket.definition.kind,
    ],
  },
);
