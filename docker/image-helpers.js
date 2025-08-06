import { execSync } from "node:child_process";
import fs from "node:fs";
import { dirname } from "node:path";
import { chdir } from "node:process";
import { promisify } from "node:util";

const sleep = promisify(setTimeout);

// These are lazy globals
let toml;
let core;
let crane;

export function pnpmInstall() {
  // change workdir to the root of the repo
  chdir(dirname(process.argv[1]) + "/..");
  // install repo pnpm dependencies
  execSync("pnpm install --frozen-lockfile", { stdio: "inherit" });
}

export async function installCrane() {
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
  return crane;
}

// This is kinda gross but means we can just run the script directly and it will
// work without running pnpm install before hand...
export async function lazyImports() {
  await import("zx/globals");
  toml = await import("toml");
  core = await import("@actions/core");
}

export async function assertExecutingInRepoRoot() {
  const gitRoot = (await $`git rev-parse --show-toplevel`).toString().trim();
  const currentDir = process.cwd();
  if (gitRoot !== currentDir) {
    console.error(chalk.red(`ERROR: must execute this script from the root of the repo: ${gitRoot}`));
    process.exit(1);
  }
}

export async function waitForImageToBecomeAvailable(imageToWaitFor, waitForImageSeconds) {
  const WAIT_TIME_IN_BETWEEN_ATTEMPTS = 10000; // 10 seconds in ms
  const startTimeMs = Date.now();
  function timeElapsedSeconds() {
    return (Date.now() - startTimeMs) / 1000;
  }
  $.verbose = false; // disable verbose output from zx
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

export const Environment = {
  CI: "ci",
  LOCAL: "local",
  TEST: "test",
};


export function getEnvironment() {
  if (process.env.CI === "true") {
    return Environment.CI;
  } else if (import.meta.jest !== undefined) {
    return Environment.TEST;
  } else {
    return Environment.LOCAL;
  }
}

export const CargoBuildFeatures = {
  Default: "default",
  Failpoints: "failpoints",
};

export const CargoBuildProfiles = {
  Release: "release",
  Performance: "performance",
}

export function getImagesToWaitFor(releaseDefaultOnly) {
  const perfImages = ["validator", "validator-testing"];
  const images = ["forge", "tools", "indexer-grpc"];
  const imagesToWaitFor = {};
  for (const image of [...perfImages, ...images]) {
    imagesToWaitFor[image] = {
      [CargoBuildProfiles.Release]: releaseDefaultOnly || !perfImages.includes(image) ? [
        CargoBuildFeatures.Default,
      ] : [
        CargoBuildFeatures.Default,
        CargoBuildFeatures.Failpoints,
      ],
    };

    if (!releaseDefaultOnly && perfImages.includes(image)) {
      imagesToWaitFor[image][CargoBuildProfiles.Performance] = [
        CargoBuildFeatures.Default,
      ];
    }
  }
  return imagesToWaitFor;
}

export function parseArgsFromFlagOrEnv(requiredArgs, optionalArgs, booleanArgs) {
  const parsedArgs = {};

  for (const arg of requiredArgs) {
    const argValue = argv[arg.toLowerCase().replaceAll("_", "-")] ?? process.env[arg];
    if (!argValue) {
      console.error(chalk.red(`ERROR: Missing required argument or environment variable: ${arg}`));
      process.exit(1);
    }
    parsedArgs[arg] = argValue;
  }

  for (const arg of optionalArgs) {
    const argValue = argv[arg.toLowerCase().replaceAll("_", "-")] ?? process.env[arg];
    parsedArgs[arg] = argValue;
  }

  for (const arg of booleanArgs) { 
    const argExists = (argv[arg.toLowerCase().replaceAll("_", "-")] !== undefined) || 
    (process.env[arg] === "true");
    parsedArgs[arg] = argExists;
  }

  return parsedArgs;
}

// joinTagSegments joins tag segments with a dash, but only if the segment is not empty
export function joinTagSegments(...segments) {
  return segments.filter((s) => s).join("_");
}

export function assertTagMatchesSourceVersion(imageTag) {
  const config = toml.parse(fs.readFileSync("aptos-node/Cargo.toml"));
  const configVersion = config.package.version;
  if (!doesTagMatchConfig(imageTag, configVersion)) {
    reportError(`image tag does not match cargo version: ${imageTag} !== ${configVersion}`, { throwOnFailure: true });
  }
}

const APTOS_RELEASE_REGEX = /aptos-node-v(\d+\.\d+\.\d+)/;

function doesTagMatchConfig(imageTag, configVersion) {
  if (!APTOS_RELEASE_REGEX.test(imageTag)) {
    reportError(`image tag does not match cargo version: ${imageTag} !== ${configVersion}`, { throwOnFailure: true });
  }
  const version = imageTag.match(APTOS_RELEASE_REGEX)[1];
  return version === configVersion;
}


function reportError(message, opts = { throwOnFailure: false }) {
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

export function isReleaseImage(imageTag) {
  return APTOS_RELEASE_REGEX.test(imageTag);
}
