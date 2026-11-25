import {
  type ConnectMcpApprovalCall,
  createConnectMcpApprovalCommand,
} from "@mycode/client-runtime/state/connectMcpApproval";
import { runAtomCommand, squashAtomCommandFailure } from "@mycode/client-runtime/state/runtime";
import type { EnvironmentId } from "@mycode/contracts";
import * as Result from "effect/Result";

import { connectionAtomRuntime } from "../connection/runtime";
import { appAtomRegistry } from "../rpc/atomRegistry";

const connectMcpApproval = createConnectMcpApprovalCommand(connectionAtomRuntime);

/** One agent sign-in call to `environmentId`, sent with this account's MyCode Connect session. */
export async function runConnectMcpApproval(
  environmentId: EnvironmentId,
  call: ConnectMcpApprovalCall,
) {
  const result = await runAtomCommand(
    appAtomRegistry,
    connectMcpApproval,
    { environmentId, call },
    { reportFailure: false },
  );
  return result._tag === "Success"
    ? Result.succeed(result.value)
    : Result.fail(squashAtomCommandFailure(result));
}
