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

import {
  assertExecutingInRepoRoot,
  CargoBuildFeatures,
  CargoBuildProfiles,
  Environment,
  getEnvironment,
  installCrane,
  lazyImports,
  parseArgsFromFlagOrEnv,
  pnpmInstall,
  waitForImageToBecomeAvailable,
  joinTagSegments,
} from "./image-helpers.js";

// When we release aptos-node, we also want to release related images for tooling, testing, etc. Similarly, other images have other related images
// that we can release together, ie in a release group.
const IMAGES_TO_RELEASE_BY_RELEASE_GROUP = {
  "aptos-node": ["validator", "validator-testing", "faucet", "tools"],
  "aptos-indexer-grpc": ["indexer-grpc"],
};

const IMAGE_NAMES_TO_RELEASE_ONLY_INTERNAL = ["validator-testing"];
const IMAGES_TO_RELEASE = {
  validator: {
    [CargoBuildProfiles.Performance]: [CargoBuildFeatures.Default],
    [CargoBuildProfiles.Release]: [CargoBuildFeatures.Default],
  },
  "validator-testing": {
    [CargoBuildProfiles.Performance]: [CargoBuildFeatures.Default],
    [CargoBuildProfiles.Release]: [CargoBuildFeatures.Default],
  },
  faucet: {
    [CargoBuildProfiles.Release]: [CargoBuildFeatures.Default],
  },
  forge: {
    [CargoBuildProfiles.Release]: [CargoBuildFeatures.Default],
  },
  tools: {
    [CargoBuildProfiles.Release]: [CargoBuildFeatures.Default],
  },
  "node-checker": {
    [CargoBuildProfiles.Release]: [CargoBuildFeatures.Default],
  },
  "indexer-grpc": {
    [CargoBuildProfiles.Release]: [CargoBuildFeatures.Default],
  },
};

async function main() {
  const REQUIRED_ARGS = ["GIT_SHA", "GCP_DOCKER_ARTIFACT_REPO", "AWS_ACCOUNT_ID", "IMAGE_TAG_PREFIX"];
  const OPTIONAL_ARGS = ["WAIT_FOR_IMAGE_SECONDS", "DRY_RUN"];
  const BOOLEAN_ARGS = ["PROFILE_RELEASE"];

  const parsedArgs = parseArgsFromFlagOrEnv(REQUIRED_ARGS, OPTIONAL_ARGS, BOOLEAN_ARGS);

  await assertExecutingInRepoRoot();
  const crane = await installCrane();
  const craneVersion = await $`${crane} version`;
  console.log(`INFO: crane version: ${craneVersion}`);

  const AWS_ECR = `${parsedArgs.AWS_ACCOUNT_ID}.dkr.ecr.us-west-2.amazonaws.com/aptos`;
  const GCP_ARTIFACT_REPO = parsedArgs.GCP_DOCKER_ARTIFACT_REPO;
  const DOCKERHUB = "docker.io/aptoslabs";

  const INTERNAL_TARGET_REGISTRIES = [GCP_ARTIFACT_REPO, AWS_ECR];

  const ALL_TARGET_REGISTRIES = [...INTERNAL_TARGET_REGISTRIES, DOCKERHUB];

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
        const featureSuffix = feature === CargoBuildFeatures.Default ? "" : feature;
        const targetRegistries = IMAGE_NAMES_TO_RELEASE_ONLY_INTERNAL.includes(image)
          ? INTERNAL_TARGET_REGISTRIES
          : ALL_TARGET_REGISTRIES;

        for (const targetRegistry of targetRegistries) {
          const imageSource = `${parsedArgs.GCP_DOCKER_ARTIFACT_REPO}/${image}:${joinTagSegments(
            profilePrefix,
            featureSuffix,
            parsedArgs.GIT_SHA,
          )}`;
          const imageTarget = `${targetRegistry}/${image}:${joinTagSegments(
            parsedArgs.IMAGE_TAG_PREFIX,
            profilePrefix,
            featureSuffix,
          )}`;
          await waitForImageToBecomeAvailable(imageSource, parsedArgs.WAIT_FOR_IMAGE_SECONDS);
          if (parsedArgs.DRY_RUN) {
            console.info(chalk.yellow(`INFO: skipping copy of ${imageSource} to ${imageTarget} due to dry run`));
            continue;
          } else {
            console.info(chalk.green(`INFO: copying ${imageSource} to ${imageTarget}`));
          }
          await $`${crane} copy ${imageSource} ${imageTarget}`;
          await $`${crane} copy ${imageSource} ${joinTagSegments(imageTarget, parsedArgs.GIT_SHA)}`;
        }
      }
    }
  }
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

// This prevents tests from executing main
if (import.meta.jest === undefined) {
  pnpmInstall();
  await lazyImports();
  await main();
} else {
  // Because we do this weird import in order to test we also have to resolve imports
  // However we force the caller to actually install pnpm first here
  await lazyImports();
}
