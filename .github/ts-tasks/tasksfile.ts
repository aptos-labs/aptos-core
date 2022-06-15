import { cli } from "tasksfile";
import { pruneGithubWorkflowRuns } from "./prune-github-workflow-runs";

cli({
  pruneGithubWorkflowRuns,
});
