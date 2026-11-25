import * as Exit from "effect/Exit";
import * as Schema from "effect/Schema";

import { MyCodeProjectFile, T3_PROJECT_FILE_SCHEMA_URL } from "@mycode/contracts";

import { fromLenientJson } from "./schemaJson.ts";

export const MyCodeProjectFileFromJson = fromLenientJson(MyCodeProjectFile);

const decodeMyCodeProjectFile = Schema.decodeExit(MyCodeProjectFileFromJson);

export function parseMyCodeProjectFile(contents: string): MyCodeProjectFile | null {
  const decoded = decodeMyCodeProjectFile(contents);
  return Exit.isSuccess(decoded) ? decoded.value : null;
}

export function buildMyCodeProjectFileJsonSchema(): Record<string, unknown> {
  const document = Schema.toJsonSchemaDocument(MyCodeProjectFile, { onExcessProperty: "error" });
  const jsonSchema: Record<string, unknown> = {
    $schema: "https://json-schema.org/draft/2020-12/schema",
    $id: T3_PROJECT_FILE_SCHEMA_URL,
    ...document.schema,
  };
  if (document.definitions && Object.keys(document.definitions).length > 0) {
    jsonSchema.$defs = document.definitions;
  }
  return jsonSchema;
}
