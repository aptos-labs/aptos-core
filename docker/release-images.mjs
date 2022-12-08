#!/usr/bin/env -S node

// This script releases the main aptos docker images to docker hub.
// It does so by copying the images from aptos GCP artifact registry to docker hub.
// It also copies the release tags to GCP Artifact Registry and AWS ECR.
//
// Usually it's run in CI, but you can also run it locally in emergency situations, assuming you have the right credentials.
// Before you run this locally, check one more time whether you can trigger a CI build instead which is usually easier and safer.
// You can do so via the Github UI or CLI:
// E.g: gh workflow run copy-images-to-dockerhub.yaml --ref <branch_or_tag> -F image_tag_prefix=release_testing
//
// If that doesn't work for you, you can run this script locally:
//
// Prerequisites when running locally:
// 1. Tools:
//  - docker
//  - gcloud
//  - aws cli
//  - node (node.js)
//  - crane - https://github.com/google/go-containerregistry/tree/main/cmd/crane#installation
//  - pnpm - https://pnpm.io/installation
// 2. docker login - with authorization to push to the `aptoslabs` org
// 3. gcloud auth configure-docker us-west1-docker.pkg.dev
// 4. gcloud auth login --update-adc
// 5. AWS CLI credentials configured
//
// Once you have all prerequisites fulfilled, you can run this script via:
// GIT_SHA=${{ github.sha }} GCP_DOCKER_ARTIFACT_REPO="${{ secrets.GCP_DOCKER_ARTIFACT_REPO }}" AWS_ACCOUNT_ID="${{ secrets.AWS_ECR_ACCOUNT_NUM }}" IMAGE_TAG_PREFIX="${{ inputs.image_tag_prefix }}" ./docker/release_images.sh --wait-for-image-seconds=1800

const Features = {
  Default: "default",
  Indexer: "indexer",
};

const IMAGES_TO_RELEASE = {
  validator: {
    performance: [
      Features.Default,
    ],
    release: [
      Features.Default,
      Features.Indexer,
    ],
  },
  forge: {
    release: [
      Features.Default,
    ],
  },
  tools: {
    release: [
      Features.Default,
    ],
  },
  "node-checker": {
    release: [
      Features.Default,
    ],
  }
};

import { execSync } from "node:child_process";
import { dirname } from "node:path";
import { chdir } from "node:process";
import { promisify } from "node:util";
const sleep = promisify(setTimeout);

chdir(dirname(process.argv[1]) + "/.."); // change workdir to the root of the repo
// install repo pnpm dependencies
execSync("pnpm install --frozen-lockfile", { stdio: "inherit" });
await import("zx/globals");

const REQUIRED_ARGS = ["GIT_SHA", "GCP_DOCKER_ARTIFACT_REPO", "AWS_ACCOUNT_ID", "IMAGE_TAG_PREFIX"];
const OPTIONAL_ARGS = ["WAIT_FOR_IMAGE_SECONDS"];

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
  await $`curl -sL https://github.com/google/go-containerregistry/releases/download/v0.11.0/go-containerregistry_Linux_x86_64.tar.gz > crane.tar.gz`;
  await $`tar -xf crane.tar.gz`;
  const sha = (await $`shasum -a 256 ./crane | awk '{ print $1 }'`).toString().trim();
  if (sha !== "2af448965b5feb6c315f4c8e79b18bd15f8c916ead0396be3962baf2f0c815bf") {
    console.error(chalk.red(`ERROR: sha256 mismatch for crane- got: ${sha}`));
    process.exit(1);
  }
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

const TARGET_REGISTRIES = [
  parsedArgs.GCP_DOCKER_ARTIFACT_REPO,
  "docker.io/aptoslabs",
  `${parsedArgs.AWS_ACCOUNT_ID}.dkr.ecr.us-west-2.amazonaws.com/aptos`,
];

// default 10 seconds
parsedArgs.WAIT_FOR_IMAGE_SECONDS = parseInt(parsedArgs.WAIT_FOR_IMAGE_SECONDS ?? 10, 10);

for (const [image, imageConfig] of Object.entries(IMAGES_TO_RELEASE)) {
  for (const [profile, features] of Object.entries(imageConfig)) {
    // build profiles that are not the default "release" will have a separate prefix
    const profilePrefix = profile === "release" ? "" : profile;
    for (const feature of features) {
      const featureSuffix = feature === Features.Default ? "" : feature;

      for (const targetRegistry of TARGET_REGISTRIES) {
        const imageSource = `${parsedArgs.GCP_DOCKER_ARTIFACT_REPO}/${image}:${joinTagSegments(
          profilePrefix,
          featureSuffix,
          parsedArgs.GIT_SHA,
        )}`;
        const imageTarget = `${targetRegistry}/${image}:${joinTagSegments(parsedArgs.IMAGE_TAG_PREFIX, profilePrefix, featureSuffix)}`;
        console.info(chalk.green(`INFO: copying ${imageSource} to ${imageTarget}`));
        await waitForImageToBecomeAvailable(imageSource, parsedArgs.WAIT_FOR_IMAGE_SECONDS);
        await $`${crane} copy ${imageSource} ${imageTarget}`;
        await $`${crane} copy ${imageSource} ${joinTagSegments(imageTarget, parsedArgs.GIT_SHA)}`;
      }
    }
  }
}

// joinTagSegments joins tag segments with a dash, but only if the segment is not empty
function joinTagSegments(...segments) {
  return segments.filter((s) => s).join("_");
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
