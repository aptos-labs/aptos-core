#!/usr/bin/env -S node

// This script waits for the images to be available in the GCP artifact registry that are needed for a CI job.
// These images are typically built on push to the main branch or on a PR, from the "docker-build-test.yaml" workflow.

// Try it out:
// GCP_DOCKER_ARTIFACT_REPO=us-docker.pkg.dev/aptos-registry/docker GIT_SHA=$(git fetch && git rev-parse origin/main) ./docker/wait-images-ci.mjs --wait-for-image-seconds=3600 --release-only
import {
  assertExecutingInRepoRoot,
  CargoBuildFeatures,
  CargoBuildProfiles,
  installCrane,
  lazyImports,
  parseArgsFromFlagOrEnv,
  pnpmInstall,
  waitForImageToBecomeAvailable,
  joinTagSegments,
} from "./image-helpers.js";

const IMAGES_TO_WAIT_FOR = {
  validator: {
    [CargoBuildProfiles.Performance]: [
      CargoBuildFeatures.Default,
    ],
    [CargoBuildProfiles.Release]: [
      CargoBuildFeatures.Default,
      CargoBuildFeatures.Failpoints,
    ],
  },
  "validator-testing": {
    [CargoBuildProfiles.Performance]: [
      CargoBuildFeatures.Default,
    ],
    [CargoBuildProfiles.Release]: [
      CargoBuildFeatures.Default,
      CargoBuildFeatures.Failpoints,
    ],
  },
  forge: {
    [CargoBuildProfiles.Release]: [
      CargoBuildFeatures.Default,
    ],
  },
  tools: {
    [CargoBuildProfiles.Release]: [
      CargoBuildFeatures.Default,
    ],
  },
  "indexer-grpc": {
    [CargoBuildProfiles.Release]: [
      CargoBuildFeatures.Default,
    ],
  },
};


async function main() {
  const REQUIRED_ARGS = ["GIT_SHA", "GCP_DOCKER_ARTIFACT_REPO"];
  const OPTIONAL_ARGS = ["WAIT_FOR_IMAGE_SECONDS"];
  const BOOLEAN_ARGS = ["RELEASE_ONLY"];

  const parsedArgs = parseArgsFromFlagOrEnv(REQUIRED_ARGS, OPTIONAL_ARGS, BOOLEAN_ARGS);

  await assertExecutingInRepoRoot();
  await installCrane();

  const GCP_ARTIFACT_REPO = parsedArgs.GCP_DOCKER_ARTIFACT_REPO;

  // default 10 seconds
  parsedArgs.WAIT_FOR_IMAGE_SECONDS = parseInt(parsedArgs.WAIT_FOR_IMAGE_SECONDS ?? 10, 10);

  // iterate over all images to wait for
  for (const [image, imageConfig] of Object.entries(IMAGES_TO_WAIT_FOR)) {
    for (const [profile, features] of Object.entries(imageConfig)) {
      // build profiles that are not the default "release" will have a separate prefix
      const profilePrefix = profile === CargoBuildProfiles.Release ? "" : profile;
      for (const feature of features) {
        const featureSuffix = feature === CargoBuildFeatures.Default ? "" : feature;
        const imageSource = `${parsedArgs.GCP_DOCKER_ARTIFACT_REPO}/${image}:${joinTagSegments(
          profilePrefix,
          featureSuffix,
          parsedArgs.GIT_SHA,
        )}`;
        await waitForImageToBecomeAvailable(imageSource, parsedArgs.WAIT_FOR_IMAGE_SECONDS);
      }
    }
  }
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
