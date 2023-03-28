const path = require("path");
const { execSync } = require("child_process");

const ANS_CORE_FOLDER = "/aptos-names-contracts/core";
const APTOS_INIT_COMMAND = "aptos init --network local";
const GET_DEFAULT_PROFILE_COMMAND = "aptos config show-profiles --profile default";

/**
 * TS SDK supports ANS. Since ANS contract is not part of aptos-framework
 * we need to get the ANS contract, publish it to local testnet and test against it.
 * This script clones the aptos-names-contracts repo {@link https://github.com/aptos-labs/aptos-names-contracts},
 * creates a default profile using `aptos init` and then use that profile to publish the contract
 * to the local testnet.
 * After the contract is published, we delete the cloned repo folder.
 *
 * We run this script whenever we run `pnpm test` in the TS SDK.
 */

try {
  // delete aptos-names-contracts folder
  console.log("---deleting aptos-names-contracts folder---");
  deleteAnsFolder();
  // 1. Clone ANS repository into the current directory
  console.log("---clone ANS repository---");
  execSync("git clone https://github.com/aptos-labs/aptos-names-contracts.git", {
    cwd: path.resolve(__dirname, ""),
  });

  // 2. initialize a default profile
  console.log("---initialize a default profile---");
  execSync(APTOS_INIT_COMMAND, {
    cwd: __dirname + ANS_CORE_FOLDER,
  });

  // 3. get default profile info
  console.log("---get default profile info---");
  const data = execSync(GET_DEFAULT_PROFILE_COMMAND, {
    cwd: __dirname + ANS_CORE_FOLDER,
  })
    .toString()
    .trim();

  // 4. get default profile account address
  console.log("---get default profile account address---");
  const profileAccountAddress = JSON.parse(data).Result.default.account;

  // 5. publish ans modules under the default profile
  console.log("---publish ans modules---");
  execSync(
    `aptos move publish --named-addresses aptos_names=0x${profileAccountAddress},aptos_names_admin=0x${profileAccountAddress},aptos_names_funds=0x${profileAccountAddress} --assume-yes`,
    {
      cwd: __dirname + ANS_CORE_FOLDER,
    },
  );

  // 6. Delete aptos-names-contracts folder created by the git clone command
  console.log("---module published, deleting aptos-names-contracts folder---");
  deleteAnsFolder();
} catch (error: any) {
  console.error("An error occurred:");
  console.error("error", error);
  deleteAnsFolder();
  process.exit(1);
}

function deleteAnsFolder() {
  execSync("rm -rf aptos-names-contracts", {
    cwd: path.resolve(__dirname, ""),
  });
}
