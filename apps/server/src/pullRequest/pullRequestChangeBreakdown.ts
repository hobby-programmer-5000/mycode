import type { PullRequestChangeBreakdown } from "@mycode/contracts";
import { summarizeChangedFiles } from "@mycode/shared/changedFileKind";
import type { ProviderFileStats } from "@mycode/source-control-core/server/PullRequestProvider";

export function toChangeBreakdown(stats: ProviderFileStats): PullRequestChangeBreakdown {
  if (stats.truncated) return null;
  return summarizeChangedFiles(stats.files);
}
