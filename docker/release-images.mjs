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
// 3. gcloud auth configure-docker us-docker.pkg.dev
// 4. gcloud auth login --update-adc
// 5. AWS CLI credentials configured
//
// Once you have all prerequisites fulfilled, you can run this script via:
// GIT_SHA=${{ github.sha }} GCP_DOCKER_ARTIFACT_REPO="${{ vars.GCP_DOCKER_ARTIFACT_REPO }}" AWS_ACCOUNT_ID="${{ secrets.AWS_ECR_ACCOUNT_NUM }}" IMAGE_TAG_PREFIX="${{ inputs.image_tag_prefix }}" ./docker/release_images.sh --wait-for-image-seconds=1800
//
//
// You can also run this script locally with the DRY_RUN flag to test it out:
// IMAGE_TAG_PREFIX=devnet AWS_ACCOUNT_ID=bla GCP_DOCKER_ARTIFACT_REPO=bla GIT_SHA=bla ./docker/release-images.mjs --wait-for-image-seconds=3600 --dry-run
//
// You can also run unittests by running docker/__tests__/release-images.test.js

// When we release aptos-node, we also want to release related images for tooling, testing, etc. Similarly, other images have other related images
// that we can release together, ie in a release group.
const IMAGES_TO_RELEASE_BY_RELEASE_GROUP = {
  "aptos-node": [
    "validator",
    "validator-testing",
    "faucet",
    "tools",
  ],
  "aptos-indexer-grpc": [
    "indexer-grpc",
  ],
}

const Features = {
  Default: "default",
};
const IMAGES_TO_RELEASE_ONLY_INTERNAL = ["validator-testing"];
const IMAGES_TO_RELEASE = {
  validator: {
    performance: [
      Features.Default,
    ],
    release: [
      Features.Default,
    ],
  },
  "validator-testing": {
    performance: [
      Features.Default,
    ],
    release: [
      Features.Default,
    ],
  },
  faucet: {
    release: [
      Features.Default,
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
  },
  "indexer-grpc": {
    release: [
      Features.Default,
    ],
  },
};

import { execSync } from "node:child_process";
import { dirname } from "node:path";
import { chdir } from "node:process";
import { promisify } from "node:util";
import fs from "node:fs";

const sleep = promisify(setTimeout);

// These are lazy globals
let toml;
let core;
let crane;

function pnpmInstall() {
  // change workdir to the root of the repo
  chdir(dirname(process.argv[1]) + "/..");
  // install repo pnpm dependencies
  execSync("pnpm install --frozen-lockfile", { stdio: "inherit" });
}

async function installCrane() {
  if (getEnvironment() === Environment.CI) {
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
}

// This is kinda gross but means we can just run the script directly and it will
// work without running pnpm install before hand...
async function lazyImports() {
  await import("zx/globals");
  toml = await import("toml");
  core = await import("@actions/core");
}

const Environment = {
  CI: "ci",
  LOCAL: "local",
  TEST: "test",
};


function getEnvironment() {
  if (process.env.CI === "true") {
    return Environment.CI;
  } else if (import.meta.jest !== undefined) {
    return Environment.TEST;
  } else {
    return Environment.LOCAL;
  }
}

function reportError(message, opts={throwOnFailure: false}) {
  if (getEnvironment() === Environment.CI) {
    core.setFailed(message);
  } else if (getEnvironment() === Environment.LOCAL) {
    console.error(message);
  } else if (getEnvironment() === Environment.TEST) {
    // Errors in tests are expected and mess up formatting
    console.log(message);
  }
  if (opts.throwOnFailure) {
    throw new Error(message);
  }
}

async function main() {
  const REQUIRED_ARGS = ["GIT_SHA", "GCP_DOCKER_ARTIFACT_REPO", "AWS_ACCOUNT_ID", "IMAGE_TAG_PREFIX"];
  const OPTIONAL_ARGS = ["WAIT_FOR_IMAGE_SECONDS", "DRY_RUN"];

  const parsedArgs = {};

  await assertExecutingInRepoRoot();

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

  await installCrane();

  const AWS_ECR = `${parsedArgs.AWS_ACCOUNT_ID}.dkr.ecr.us-west-2.amazonaws.com/aptos`;
  const GCP_ARTIFACT_REPO = parsedArgs.GCP_DOCKER_ARTIFACT_REPO;
  const DOCKERHUB = "docker.io/aptoslabs";

  const INTERNAL_TARGET_REGISTRIES = [GCP_ARTIFACT_REPO, AWS_ECR];

  const ALL_TARGET_REGISTRIES = [
    ...INTERNAL_TARGET_REGISTRIES,
    DOCKERHUB,
  ];

  // default 10 seconds
  parsedArgs.WAIT_FOR_IMAGE_SECONDS = parseInt(parsedArgs.WAIT_FOR_IMAGE_SECONDS ?? 10, 10);

  // dry run
  console.log(`INFO: dry run: ${parsedArgs.DRY_RUN}`);

  // get the appropriate release group based on the image tag prefix
  const imageReleaseGroup = getImageReleaseGroupByImageTagPrefix(parsedArgs.IMAGE_TAG_PREFIX);
  console.log(`INFO: image release group: ${imageReleaseGroup}`);

  // only release the images that are part of the release group
  const imageNamesToRelease = IMAGES_TO_RELEASE_BY_RELEASE_GROUP[imageReleaseGroup];
  console.log(`INFO: image names to release: ${JSON.stringify(imageNamesToRelease)}`);

  // iterate over all images to release, including their release configurations
  const imagesToRelease = {};
  for (const imageName of imageNamesToRelease) {
    imagesToRelease[imageName] = IMAGES_TO_RELEASE[imageName];
  }
  for (const [image, imageConfig] of Object.entries(imagesToRelease)) {
    for (const [profile, features] of Object.entries(imageConfig)) {
      // build profiles that are not the default "release" will have a separate prefix
      const profilePrefix = profile === "release" ? "" : profile;
      for (const feature of features) {
        const featureSuffix = feature === Features.Default ? "" : feature;
        const targetRegistries = IMAGES_TO_RELEASE_ONLY_INTERNAL.includes(image) ? INTERNAL_TARGET_REGISTRIES : ALL_TARGET_REGISTRIES;

        for (const targetRegistry of targetRegistries) {
          const imageSource = `${parsedArgs.GCP_DOCKER_ARTIFACT_REPO}/${image}:${joinTagSegments(
            profilePrefix,
            featureSuffix,
            parsedArgs.GIT_SHA,
          )}`;
          const imageTarget = `${targetRegistry}/${image}:${joinTagSegments(parsedArgs.IMAGE_TAG_PREFIX, profilePrefix, featureSuffix)}`;
          console.info(chalk.green(`INFO: copying ${imageSource} to ${imageTarget}`));
          if (parsedArgs.DRY_RUN) {
            continue;
          }
          await waitForImageToBecomeAvailable(imageSource, parsedArgs.WAIT_FOR_IMAGE_SECONDS);
          await $`${crane} copy ${imageSource} ${imageTarget}`;
          await $`${crane} copy ${imageSource} ${joinTagSegments(imageTarget, parsedArgs.GIT_SHA)}`;
        }
      }
    }
  }
}

async function assertExecutingInRepoRoot() {
  const gitRoot = (await $`git rev-parse --show-toplevel`).toString().trim();
  const currentDir = process.cwd();
  if (gitRoot !== currentDir) {
    console.error(chalk.red(`ERROR: must execute this script from the root of the repo: ${gitRoot}`));
    process.exit(1);
  }
}

// joinTagSegments joins tag segments with a dash, but only if the segment is not empty
function joinTagSegments(...segments) {
  return segments.filter((s) => s).join("_");
}

// The image tag prefix is used to determine the release group. Examples:
// * tag a release as "aptos-node-vX.Y.Z"
// * tag a release as "aptos-indexer-grpc-vX.Y.Z"
export function getImageReleaseGroupByImageTagPrefix(prefix) {
  // iterate over the keys in IMAGES_TO_RELEASE_BY_RELEASE_GROUP
  // if the prefix includes the release group, then return the release group
  for (const [imageReleaseGroup, imagesToRelease] of Object.entries(IMAGES_TO_RELEASE_BY_RELEASE_GROUP)) {
    if (prefix.includes(imageReleaseGroup)) {
      return imageReleaseGroup;
    }
  }
  // if there's no match, then release aptos-node by default
  return "aptos-node";
}

const APTOS_RELEASE_REGEX = /aptos-node-v(\d+\.\d+\.\d+)/;

export function assertTagMatchesSourceVersion(imageTag) {
  const config = toml.parse(fs.readFileSync("aptos-node/Cargo.toml"));
  const configVersion = config.package.version;
  if (!doesTagMatchConfig(imageTag, configVersion)) {
    reportError(`image tag does not match cargo version: ${imageTag} !== ${configVersion}`, {throwOnFailure: true});
  }
}

export function doesTagMatchConfig(imageTag, configVersion) {
  if (!APTOS_RELEASE_REGEX.test(imageTag)) {
    reportError(`image tag does not match cargo version: ${imageTag} !== ${configVersion}`, {throwOnFailure: true});
  }
  const version = imageTag.match(APTOS_RELEASE_REGEX)[1];
  return version === configVersion;
}

export function isReleaseImage(imageTag) {
  return APTOS_RELEASE_REGEX.test(imageTag);
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

// This prevents tests from executing main
if (import.meta.jest === undefined) {
  pnpmInstall();
  await lazyImports();
  await main()
} else {
  // Because we do this weird import in order to test we also have to resolve imports
  // However we force the caller to actually install pnpm first here
  await lazyImports();
}
