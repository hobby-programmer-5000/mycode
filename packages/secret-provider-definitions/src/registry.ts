import { makeSecretProviderDefinitionRegistry } from "@mycode/secret-provider-core/shared/definition";
import * as OnePassword from "@mycode/secret-provider-onepassword/shared/definition";

export const builtInSecretProviderDefinitions = makeSecretProviderDefinitionRegistry([
  OnePassword.definition,
]);
