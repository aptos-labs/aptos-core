const { execSync } = require("child_process");
require("dotenv").config();

/**
 * TS SDK supports ANS. Since ANS contract is not part of aptos-framework
 * we need to get the ANS contract, publish it to local testnet and test against it.
 * This script clones the aptos-names-contracts repo {@link https://github.com/aptos-labs/aptos-names-contracts},
 * uses a pre created account address and private key to fund that account and
 * then publish the contract under that account.
 * After the contract is published, we delete the cloned repo folder.
 *
 * This script runs when testing locally and on CI (as part of sdk-release.yaml) using `pnpm test`.
 */

// URLs at which the node and faucet are running.
const APTOS_NODE_URL = process.env.APTOS_NODE_URL;
const APTOS_FAUCET_URL = process.env.APTOS_FAUCET_URL;

// Env vars that configure how we run the CLI.
const DOCKER_IMAGE = process.env.DOCKER_IMAGE;

// ANS account we use to publish the contract
const ANS_TEST_ACCOUNT_PRIVATE_KEY =
  process.env.ANS_TEST_ACCOUNT_PRIVATE_KEY || "0x37368b46ce665362562c6d1d4ec01a08c8644c488690df5a17e13ba163e20221";
const ANS_TEST_ACCOUNT_ADDRESS =
  process.env.ANS_TEST_ACCOUNT_ADDRESS || "0x585fc9f0f0c54183b039ffc770ca282ebd87307916c215a3e692f2f8e4305e82";

try {
  // 0. Create a temporary directory to clone the repo into. Note: For this to work in
  // CI, it is essential that TMPDIR is set to a directory that can actually be mounted.
  // Learn more here: https://stackoverflow.com/a/76523941/3846032.
  console.log("---creating temporary directory for ANS code---");
  let tempDir = execSync("mktemp -d").toString("utf8").trim();

  // 1. Clone the ANS repo into the temporary directory.
  console.log(`---cloning ANS repository to ${tempDir}---`);
  execSync(`git clone https://github.com/aptos-labs/aptos-names-contracts.git ${tempDir}`, {
    stdout: "inherit",
  });

  // The command we use to run the CLI.
  let cliInvocation;
  // Where the CLI should look to find the ANS repo.
  let repoDir;

  if (DOCKER_IMAGE) {
    // If we're using a docker image we mount the temp dir into the container.
    console.log("---running CLI using docker image---");
    cliInvocation = `docker run --network host --mount=type=bind,source=${tempDir},target=/code ${DOCKER_IMAGE} aptos`;
    repoDir = "/code";
  } else {
    // If we're using a local CLI we just use the temp dir directly.
    console.log("---running CLI using local binary---");
    cliInvocation = "aptos";
    repoDir = tempDir;
  }

  // Derive the router signer address.
  const ROUTER_SIGNER = `0x${
    JSON.parse(
      execSync(
        `${cliInvocation} account derive-resource-account-address --address ${ANS_TEST_ACCOUNT_ADDRESS} --seed "ANS ROUTER" --seed-encoding utf8`,
        {
          encoding: "utf8",
        },
      ),
    ).Result
  }`;

  // 2. Fund ANS account.
  console.log("---funding account---");
  execSync(
    `${cliInvocation} account fund-with-faucet --account ${ANS_TEST_ACCOUNT_ADDRESS} --faucet-url ${APTOS_FAUCET_URL} --url ${APTOS_NODE_URL}`,
    { stdio: "inherit" },
  );

  // 3. Publish the ANS modules under the ANS account.
  console.log("---publishing ans modules---");
  execSync(
    `${cliInvocation} move publish --package-dir ${repoDir}/core --assume-yes --private-key=${ANS_TEST_ACCOUNT_PRIVATE_KEY} --named-addresses aptos_names=${ANS_TEST_ACCOUNT_ADDRESS},aptos_names_admin=${ANS_TEST_ACCOUNT_ADDRESS},aptos_names_funds=${ANS_TEST_ACCOUNT_ADDRESS},router_signer=${ROUTER_SIGNER} --url=${APTOS_NODE_URL}`,
    { stdio: "inherit" },
  );
  console.log("---module published---");
} catch (error: any) {
  console.error("An error occurred:");
  console.error("Status", error?.status);
  console.error("parsed stdout", error?.stdout?.toString("utf8"));
  console.error("parsed stderr", error?.stderr?.toString("utf8"));
  process.exit(1);
}
