#!/usr/bin/env -S node


// This script releases indexer processor images to docker hub https://github.com/aptos-labs/aptos-indexer-processors.
// It does so by copying the images from aptos GCP artifact registry to docker hub.

// Usually it's run in CI, but you can also run it locally in emergency situations, assuming you have the right credentials.
// Before you run this locally, check one more time whether you can trigger a CI build instead which is usually easier and safer.
// You can do so via the Github UI or CLI:
// E.g: gh workflow run copy-processor-images-to-dockerhub.yaml
//
// If that doesn't work for you, you can run this script locally:
//
// Prerequisites when running locally:
// 1. Tools:
//  - docker
//  - gcloud
//  - node (node.js)
//  - crane - https://github.com/google/go-containerregistry/tree/main/cmd/crane#installation
//  - pnpm - https://pnpm.io/installation
// 2. docker login - with authorization to push to the `aptoslabs` org
// 3. gcloud auth configure-docker us-west1-docker.pkg.dev
// 4. gcloud auth login --update-adc
//
// Once you have all prerequisites fulfilled, you can run this script via:
// GIT_SHA=${{ github.sha }} GCP_DOCKER_ARTIFACT_PROCESSOR_REPO_US="${{ secrets.GCP_DOCKER_ARTIFACT_REPO }}" ./docker/release-processor-images.mjs --language=rust --wait-for-image-seconds=1800


import { execSync } from "node:child_process";
import { dirname } from "node:path";
import { chdir } from "node:process";
import { promisify } from "node:util";
const sleep = promisify(setTimeout);

chdir(dirname(process.argv[1]) + "/.."); // change workdir to the root of the repo
// install repo pnpm dependencies
execSync("pnpm install --frozen-lockfile", { stdio: "inherit" });
await import("zx/globals");

const REQUIRED_ARGS = ["LANGUAGE", "GIT_SHA", "GCP_DOCKER_ARTIFACT_PROCESSOR_REPO_US"];
const OPTIONAL_ARGS = ["VERSION_TAG", "WAIT_FOR_IMAGE_SECONDS"];

const parsedArgs = {};

for (const arg of REQUIRED_ARGS) {
  const argValue = argv[arg.toLowerCase().replaceAll("_", "-")] ?? process.env[arg];
  if (!argValue) {
    console.error(chalk.red(`ERROR: Missing required argument or environment variable: ${arg}`));
    process.exit(1);
  }
  parsedArgs[arg] = argValue;
}

for (const arg of OPTIONAL_ARGS) {
  const argValue = argv[arg.toLowerCase().replaceAll("_", "-")] ?? process.env[arg];
  parsedArgs[arg] = argValue;
}

let crane;

if (process.env.CI === "true") {
  console.log("installing crane automatically in CI");
  await $`curl -sL https://github.com/google/go-containerregistry/releases/download/v0.15.1/go-containerregistry_Linux_x86_64.tar.gz > crane.tar.gz`;
  const sha = (await $`shasum -a 256 ./crane.tar.gz | awk '{ print $1 }'`).toString().trim();
  if (sha !== "d4710014a3bd135eb1d4a9142f509cfd61d2be242e5f5785788e404448a4f3f2") {
    console.error(chalk.red(`ERROR: sha256 mismatch for crane.tar.gz got: ${sha}`));
    process.exit(1);
  }
  await $`tar -xf crane.tar.gz`;
  crane = "./crane";
} else {
  if ((await $`command -v crane`.exitCode) !== 0) {
    console.log(
      chalk.red(
        "ERROR: could not find crane binary in PATH - follow https://github.com/google/go-containerregistry/tree/main/cmd/crane#installation to install",
      ),
    );
    process.exit(1);
  }
  crane = "crane";
}


function getImage(language) {
    const sourceImage = `indexer-client-examples/${language}`;
    const targetImage = `indexer-client-examples-${language}`;

    return {sourceImage, targetImage};
}

const GCP_ARTIFACT_PROCESSOR_REPO_US = parsedArgs.GCP_DOCKER_ARTIFACT_PROCESSOR_REPO_US;
const DOCKERHUB = "docker.io/aptoslabs";
const {sourceImage, targetImage} = getImage(parsedArgs.LANGUAGE);
console.log(targetImage);
// default 10 seconds
parsedArgs.WAIT_FOR_IMAGE_SECONDS = parseInt(parsedArgs.WAIT_FOR_IMAGE_SECONDS ?? 10, 10);


const imageSource = `${GCP_ARTIFACT_PROCESSOR_REPO_US}/${sourceImage}:${parsedArgs.GIT_SHA}`;
const imageGitShaTarget = `${DOCKERHUB}/${targetImage}:${parsedArgs.GIT_SHA}`;
console.info(chalk.green(`INFO: copying ${imageSource} to ${imageGitShaTarget}`));
await waitForImageToBecomeAvailable(imageSource, parsedArgs.WAIT_FOR_IMAGE_SECONDS);
await $`${crane} copy ${imageSource} ${imageGitShaTarget}`;
if(parsedArgs.VERSION_TAG !== null) {
    const imageVersionTagTarget = `${DOCKERHUB}/${targetImage}:${parsedArgs.VERSION_TAG}`;
    console.info(chalk.green(`INFO: copying ${imageSource} to ${imageVersionTagTarget}`));
    await $`${crane} copy ${imageSource} ${imageVersionTagTarget}`;
}


async function waitForImageToBecomeAvailable(imageToWaitFor, waitForImageSeconds) {
  const WAIT_TIME_IN_BETWEEN_ATTEMPTS = 10000; // 10 seconds in ms
  const startTimeMs = Date.now();
  function timeElapsedSeconds() {
    return (Date.now() - startTimeMs) / 1000;
  }
  while (timeElapsedSeconds() < waitForImageSeconds) {
    try {
      await $`${crane} manifest ${imageToWaitFor}`;
      console.info(chalk.green(`INFO: image ${imageToWaitFor} is available`));
      return;
    } catch (e) {
      if (e.exitCode === 1 && e.stderr.includes("MANIFEST_UNKNOWN")) {
        console.log(
          chalk.yellow(
            // prettier-ignore
            `WARN: Image ${imageToWaitFor} not available yet - waiting ${WAIT_TIME_IN_BETWEEN_ATTEMPTS / 1000} seconds to try again. Time elapsed: ${timeElapsedSeconds().toFixed(0,)} seconds. Max wait time: ${waitForImageSeconds} seconds`,
          ),
        );
        await sleep(WAIT_TIME_IN_BETWEEN_ATTEMPTS);
      } else {
        console.error(chalk.red(e.stderr ?? e));
        process.exit(1);
      }
    }
  }
  console.error(
    chalk.red(
      `ERROR: timed out after ${waitForImageSeconds} seconds waiting for image to become available: ${imageToWaitFor}`,
    ),
  );
  process.exit(1);
}
