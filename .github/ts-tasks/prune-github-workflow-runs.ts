import { setFailed, info, warning } from "@actions/core";
import { getOctokit } from "@actions/github";
import { create as createGlob } from "@actions/glob";
import findRepoRoot from "find-git-root";
import * as path from "path";
import * as url from "url";

const __dirname = url.fileURLToPath(new URL(".", import.meta.url));

const owner = "aptos-labs";
const repo = "aptos-core";

export async function pruneGithubWorkflowRuns() {
  const githubToken = process.env.GITHUB_TOKEN;

  if (!githubToken) {
    throw new Error("Missing environment variable `GITHUB_TOKEN`");
  }

  const ghClient = getOctokit(githubToken);

  const repoRootWithDotGit = findRepoRoot(__dirname);
  const repoRoot = repoRootWithDotGit.substring(0, repoRootWithDotGit.length - 4); // remove the `.git` suffix from the returned path

  const patterns = [`${repoRoot}/.github/worklows/*.yml`, `${repoRoot}/.github/workflows/*.yaml`];
  const globber = await createGlob(patterns.join("\n"));
  const workflowFilePaths = await globber.glob();
  const workflowFilesPresentInRepo = workflowFilePaths.map((filePath) => path.basename(filePath));

  if (workflowFilesPresentInRepo.length === 0) {
    setFailed("Found 0 workflow files under `.github/workflows` which is kinda odd - exiting early...");
    return;
  }

  info(`\nFound the following workflow files in the repo:\n${workflowFilesPresentInRepo.join("\n")}`);

  const workflowResponse = await ghClient.paginate(
    ghClient.rest.actions.listRepoWorkflows,
    {
      owner,
      repo,
    },
    (response: { data: any }) => response.data,
  );

  const obsoleteWorkflows = workflowResponse.filter(
    (workflow: { path: string }) => !workflowFilesPresentInRepo.includes(path.basename(workflow.path)),
  );

  let totalDeleted = 0;

  info(
    `
Found ${obsoleteWorkflows.length} obsolete workflows:
${obsoleteWorkflows.map((wf: { name: string; path: string }) => `'${wf.name}' - path: ${wf.path}`).join("\n")}
Deleting their workflow runs now...`,
  );

  for (const wf of obsoleteWorkflows) {
    info("Deleting workflow runs of workflow: " + wf.name);

    const workflowRuns = await ghClient.paginate(
      ghClient.rest.actions.listWorkflowRuns,
      {
        owner,
        repo,
        workflow_id: wf.id,
      },
      (response: { data: any }) => response.data,
    );

    for (const [index, run] of workflowRuns.entries()) {
      info(`Workflow: "${wf.name}" - Deleting Run (${index + 1}/${workflowRuns.length}) - Run ID: ${run.id}`);
      try {
        await ghClient.rest.actions.deleteWorkflowRun({
          owner,
          repo,
          run_id: run.id,
        });
      } catch (e: any) {
        if (e.status === 403) {
          warning(
            `Failed to delete workflow with 403 permission error: path: ${wf.path}, workflow_run_id: ${run.id}, message: ${e.message}. It's probably present in another branch. Skipping...`,
          );
          continue;
        }
        throw e;
      }
      totalDeleted++;
    }
  }

  info(`Deleted ${totalDeleted} workflow runs`);
}

// Run the function above.
pruneGithubWorkflowRuns().catch((e) => {
  info(e);
  setFailed(e);
});
