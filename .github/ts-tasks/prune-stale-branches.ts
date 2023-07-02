import core from "@actions/core";
import github from "@actions/github";
import glob from "@actions/glob";
import findRepoRoot from "find-git-root";
import * as path from "path";
import * as url from "url";
import findGitRoot from "find-git-root";
import { DateTime } from "luxon";

const __dirname = url.fileURLToPath(new URL(".", import.meta.url));

const owner = "aptos-labs";
const repo = "aptos-core";

const ignoredCommitMessages = [
  "fix trigger condition for build jobs",
  "Require permission check before running determine-docker-build-metadata",
];

export async function pruneStaleBranches() {
  const githubToken = process.env.GITHUB_TOKEN;
  if (!githubToken) {
    throw new Error("Missing environment variable `GITHUB_TOKEN`");
  }

  const repoRoot = findGitRoot(process.cwd());

  const ghClient = github.getOctokit(githubToken);

  const branches = await ghClient.paginate(ghClient.rest.repos.listBranches, {
    owner,
    repo,
  });

  core.info(`Found ${branches.length} branches in the repo`);

  const allOpenPullRequests = await ghClient.paginate(
    ghClient.rest.pulls.list,
    {
      owner,
      repo,
      state: "open",
    },
    (response) => response.data,
  );

  const staleBranches: string[] = [];

  for (const [i, branch] of branches.entries()) {
    try {
      core.startGroup(`${i}/${branches.length}: ${branch.name}`);

      // Check if branch has open PRs
      const pullRequests = allOpenPullRequests.filter((pr) => pr.head.ref === branch.name);
      if (pullRequests.length > 0) {
        core.info(`Branch ${branch.name} has ${pullRequests.length} open PRs - skipping...`);
        continue;
      }

      // Check if branch is protected
      if (branch.protected) {
        core.info(`Branch ${branch.name} is protected - skipping...`);
        continue;
      }

      // Check if commit is ignored
      const mostRecentCommitsOnBranch = await ghClient.rest.repos.listCommits({
        owner,
        repo,
        sha: branch.name,
      });

      // get most recent commit that is not ignored
      const mostRecentCommit = mostRecentCommitsOnBranch.data.find((commit) => {
        const commitMessage = commit.commit.message;
        const ignored = !ignoredCommitMessages.includes(commitMessage);
        if (ignored) {
          core.info(`Commit ${commit.sha} has message ${commitMessage} - ignoring...`);
        }
        return ignored;
      });

      if (!mostRecentCommit) {
        core.warning(`No unignored commit found on branch ${branch.name}`);
        continue;
      }

      core.info(
        "Most recent commit on branch " +
          branch.name +
          " is " +
          mostRecentCommit.sha +
          " - " +
          mostRecentCommit.commit.message,
      );

      // Check if commit is stale
      const commitDate = DateTime.fromISO(mostRecentCommit.commit.committer!.date ?? "");
      const currentDate = DateTime.now();
      const daysSinceCommit = currentDate.diff(commitDate, "days").days;

      if (daysSinceCommit > 45) {
        core.info(`Commit ${mostRecentCommit.sha} is ${daysSinceCommit.toFixed(0)} days old - deleting branch ${branch.name}`);
        staleBranches.push(branch.name);
      } else {
        core.info(`Commit ${mostRecentCommit.sha} is ${daysSinceCommit.toFixed(0)} days old - keeping...`);
      }

    } finally {
      core.endGroup();
    }
  }
  core.info(`Found the following ${staleBranches.length} stale branches:` + staleBranches.join("\n"));
}

// Run the function above.
pruneStaleBranches();
