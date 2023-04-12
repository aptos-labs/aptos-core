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

// on local publishing we want to use `aptos` commnads and on CI we want to use `docker`
const APTOS_INVOCATION = process.env.APTOS_INVOCATION || "aptos";
// environment we use when testing
const APTOS_NODE_URL = process.env.APTOS_NODE_URL;
const APTOS_FAUCET_URL = process.env.APTOS_FAUCET_URL;
// ans account we use to publish the contract
const ANS_REPO_LOCATION = process.env.ANS_REPO_LOCATION || "/tmp/ans";
const ANS_TEST_ACCOUNT_PRIVATE_KEY =
  process.env.ANS_TEST_ACCOUNT_PRIVATE_KEY || "0x37368b46ce665362562c6d1d4ec01a08c8644c488690df5a17e13ba163e20221";
const ANS_TEST_ACCOUNT_ADDRESS =
  process.env.ANS_TEST_ACCOUNT_ADDRESS || "585fc9f0f0c54183b039ffc770ca282ebd87307916c215a3e692f2f8e4305e82";

try {
  deleteAnsFolder();
  // 1. Clone ANS repository into the current directory
  console.log("---clone ANS repository---");
  execSync(`git clone https://github.com/aptos-labs/aptos-names-contracts.git ${ANS_REPO_LOCATION}`, {
    stdio: "inherit",
  });

  // 2. fund ans account
  console.log("---funding account---");
  execSync(
    `${APTOS_INVOCATION} account fund-with-faucet --account ${ANS_TEST_ACCOUNT_ADDRESS} --faucet-url ${APTOS_FAUCET_URL} --url ${APTOS_NODE_URL}`,
    { stdio: "inherit" },
  );

  // 3. publish ans modules under the ans account
  console.log("---publish ans modules---");
  execSync(
    `${APTOS_INVOCATION} move publish --package-dir /tmp/ans/core --assume-yes --private-key=${ANS_TEST_ACCOUNT_PRIVATE_KEY} --named-addresses aptos_names=0x${ANS_TEST_ACCOUNT_ADDRESS},aptos_names_admin=0x${ANS_TEST_ACCOUNT_ADDRESS},aptos_names_funds=0x${ANS_TEST_ACCOUNT_ADDRESS} --url=${APTOS_NODE_URL}`,
    { stdio: "inherit" },
  );

  // 4. Delete aptos-names-contracts folder created by the git clone command
  console.log("---module published, deleting aptos-names-contracts folder---");
  deleteAnsFolder();
} catch (error: any) {
  console.error("An error occurred:");
  console.error("parsed stdout", error.stdout.toString("utf8"));
  console.error("parsed stderr", error.stderr.toString("utf8"));
  deleteAnsFolder();
  process.exit(1);
}

function deleteAnsFolder() {
  execSync("rm -rf /tmp/ans", { stdio: "inherit" });
}
