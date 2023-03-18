const path = require("path");
const { execSync, exec, spawnSync, spawn, stdin, stdout } = require("child_process");

const ANS_CORE_FOLDER = "/aptos-names-contracts/core";
const APTOS_INIT_COMMAND = "aptos init --network local";
const GET_DEFAULT_PROFILE_COMMAND = "aptos config show-profiles --profile default";
const PUBLISH_MODULE_COMMAND = "aptos move publish --named-addresses aptos_names=";

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
execSync("rm -rf aptos-names-contracts", {
  cwd: path.resolve(__dirname, ""),
});
// // clone ans public repo
execSync(
  "git clone git@github.com:aptos-labs/aptos-names-contracts.git",
  {
    cwd: path.resolve(__dirname, ""), // path to where you want to save the file
  },
  (error: any, stdout: any, stderr: any) => {
    if (error) {
      console.error(`Error cloning repository: ${error.message}`);
      return;
    }
    if (stderr) {
      console.error(`cloning repository stderr: ${stderr}`);
      return;
    }
  },
);

// run aptos init --network local
execSync(
  `echo '\n' | ${APTOS_INIT_COMMAND}`,
  {
    cwd: __dirname + ANS_CORE_FOLDER,
  },
  (error: any, stdout: any, stderr: any) => {
    if (error) {
      console.error(`Error aptos init: ${error.message}`);
      return;
    }
    if (stderr) {
      console.error(`aptos init stderr: ${stderr}`);
      return;
    }
  },
);

// get default profile account address
const profiles = exec(
  GET_DEFAULT_PROFILE_COMMAND,
  {
    cwd: __dirname + ANS_CORE_FOLDER,
  },
  (error: any, stdout: any, stderr: any) => {
    if (error) {
      console.error(`Error show-profiles: ${error.message}`);
      return;
    }
    if (stderr) {
      console.error(`show-profiles stderr: ${stderr}`);
      return;
    }
  },
);
profiles.stdout.on("data", (data: any) => {
  const defaultProfileAddress = JSON.parse(data).Result.default.account;
  // publish ans contract to local testnet
  execSync(
    `echo '\n' | ${PUBLISH_MODULE_COMMAND}${defaultProfileAddress}`,
    {
      stdio: [0, 1, 2], // we need this so node will print the command output
      cwd: __dirname + ANS_CORE_FOLDER,
    },
    (error: any, stdout: any, stderr: any) => {
      if (error) {
        console.error(`Error publish: ${error.message}`);
        return;
      }
      if (stderr) {
        console.error(`publish stderr: ${stderr}`);
        return;
      }
    },
  );
});

execSync("rm -rf aptos-names-contracts", {
  cwd: path.resolve(__dirname, ""),
});
