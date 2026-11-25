import { T } from "gt-react";
import { cn } from "~/lib/utils";

import { CommandBlock } from "./CommandBlock";

/** Setup steps for running the local server on another computer. */
export function InstallMyCodeCommands({
  command,
  className,
}: {
  readonly command: string;
  readonly className?: string;
}) {
  return (
    <div className={cn("space-y-2", className)}>
      <p className="text-xs text-muted-foreground">
        <T>Start the server on your target machine:</T>
      </p>
      <CommandBlock command={command} />
    </div>
  );
}
