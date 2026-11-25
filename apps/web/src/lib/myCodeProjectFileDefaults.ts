import {
  T3_PROJECT_FILE_NAME,
  type EnvironmentId,
  type ProjectReadFileResult,
  type MyCodeProjectFile,
} from "@mycode/contracts";
import { parseMyCodeProjectFile } from "@mycode/shared/myCodeProjectFile";
import { executeAtomQuery } from "@mycode/client-runtime/state/runtime";
import { AsyncResult } from "effect/reactivity";

import {
  getProjectFileQueryAtom,
  resolveProjectFileQueryData,
} from "~/components/files/projectFilesQueryState";
import { appAtomRegistry } from "~/rpc/atomRegistry";

function toMyCodeProjectFile(
  environmentId: EnvironmentId,
  workspaceRoot: string,
  result: ProjectReadFileResult | null,
): MyCodeProjectFile | null {
  const data = resolveProjectFileQueryData(
    environmentId,
    workspaceRoot,
    T3_PROJECT_FILE_NAME,
    result,
  );
  if (data === null || data.exists === false || data.truncated) return null;
  return parseMyCodeProjectFile(data.contents);
}

export async function readMyCodeProjectFile(
  environmentId: EnvironmentId,
  workspaceRoot: string,
): Promise<MyCodeProjectFile | null> {
  const result = await executeAtomQuery(
    appAtomRegistry,
    getProjectFileQueryAtom(environmentId, workspaceRoot, T3_PROJECT_FILE_NAME),
    { reportDefect: false, reportFailure: false },
  );
  return toMyCodeProjectFile(
    environmentId,
    workspaceRoot,
    result._tag === "Success" ? result.value : null,
  );
}

/** The project's t3.json from an earlier read, or `undefined` when none has loaded yet. */
export function peekMyCodeProjectFile(
  environmentId: EnvironmentId,
  workspaceRoot: string,
): MyCodeProjectFile | null | undefined {
  const result = appAtomRegistry.get(
    getProjectFileQueryAtom(environmentId, workspaceRoot, T3_PROJECT_FILE_NAME),
  );
  return AsyncResult.isSuccess(result)
    ? toMyCodeProjectFile(environmentId, workspaceRoot, result.value)
    : undefined;
}
