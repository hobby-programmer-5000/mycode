import {
  T3_PROJECT_FILE_NAME,
  type EnvironmentId,
  type MyCodeProjectFile,
  type MyCodeProjectFileScript,
} from "@mycode/contracts";
import { parseMyCodeProjectFile } from "@mycode/shared/myCodeProjectFile";

import { useProjectFileQuery } from "~/components/files/projectFilesQueryState";

const NO_SCRIPTS: ReadonlyArray<MyCodeProjectFileScript> = [];

export interface MyCodeProjectFileState {
  status: "loading" | "missing" | "invalid" | "valid";
  file: MyCodeProjectFile | null;
  scripts: ReadonlyArray<MyCodeProjectFileScript>;
}

export function useMyCodeProjectFileState(
  environmentId: EnvironmentId,
  cwd: string | null,
): MyCodeProjectFileState {
  const query = useProjectFileQuery(environmentId, cwd ?? "", T3_PROJECT_FILE_NAME, cwd !== null);
  const contents = query.data && !query.data.truncated ? query.data.contents : null;
  const isPending = query.isPending;
  if (contents === null) {
    return {
      status: isPending ? "loading" : "missing",
      file: null,
      scripts: NO_SCRIPTS,
    } as const;
  }
  const file = parseMyCodeProjectFile(contents);
  if (file === null) {
    return { status: "invalid", file: null, scripts: NO_SCRIPTS } as const;
  }
  return { status: "valid", file, scripts: file.scripts ?? NO_SCRIPTS } as const;
}

export function useMyCodeProjectFileScripts(
  environmentId: EnvironmentId,
  cwd: string | null,
): ReadonlyArray<MyCodeProjectFileScript> {
  return useMyCodeProjectFileState(environmentId, cwd).scripts;
}
