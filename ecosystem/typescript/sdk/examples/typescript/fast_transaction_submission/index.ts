/* eslint-disable no-console */

import dotenv from "dotenv";
dotenv.config();

import { exit } from "process";
import { batchSubmission } from "./batch_submission";
import { multiClientSubmission } from "./multiple_client_submission";

async function main() {
  console.log("/////submiting txns in batches");
  await batchSubmission(200);

  console.log("/////submiting txns with multiple clients");
  await multiClientSubmission(50, 2);
  exit(0);
}

main();
