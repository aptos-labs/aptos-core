#!/usr/bin/env -S node

// This script waits for the images to be available in the GCP artifact registry that are needed for a CI job.
// These images are typically built on push to the main branch or on a PR, from the "docker-build-test.yaml" workflow.

// Try it out:
// GCP_DOCKER_ARTIFACT_REPO=us-docker.pkg.dev/aptos-registry/docker GIT_SHA=$(git fetch && git rev-parse origin/main) RELEASE_DEFAULT_ONLY=true ./docker/wait-images-ci.mjs --wait-for-image-seconds=3600
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
  getImagesToWaitFor,
} from "./image-helpers.js";

async function main() {
  const REQUIRED_ARGS = ["GIT_SHA", "GCP_DOCKER_ARTIFACT_REPO"];
  const OPTIONAL_ARGS = ["WAIT_FOR_IMAGE_SECONDS"];
  const BOOLEAN_ARGS = ["RELEASE_DEFAULT_ONLY"];

  const parsedArgs = parseArgsFromFlagOrEnv(REQUIRED_ARGS, OPTIONAL_ARGS, BOOLEAN_ARGS);

  await assertExecutingInRepoRoot();
  await installCrane();

  const imagesToWaitFor = getImagesToWaitFor(parsedArgs.RELEASE_DEFAULT_ONLY);
  
  // default 10 seconds
  parsedArgs.WAIT_FOR_IMAGE_SECONDS = parseInt(parsedArgs.WAIT_FOR_IMAGE_SECONDS ?? 10, 10);

  // iterate over all images to wait for
  for (const [image, imageConfig] of Object.entries(imagesToWaitFor)) {
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
