import { FaucetClient } from "../../plugins/faucet_client";
import { OpenAPIConfig } from "../../generated";

export const NODE_URL = process.env.APTOS_NODE_URL!;
export const FAUCET_URL = process.env.APTOS_FAUCET_URL!;
export const API_TOKEN = process.env.API_TOKEN!;
export const FAUCET_AUTH_TOKEN = process.env.FAUCET_AUTH_TOKEN!;

/**
 * Returns an instance of a FaucetClient with NODE_URL and FAUCET_URL from the
 * environment. If the FAUCET_AUTH_TOKEN environment variable is set, it will
 * pass that along in the header in the format the faucet expects.
 */
export function getFaucetClient(): FaucetClient {
  const config: Partial<OpenAPIConfig> = {};
  if (process.env.FAUCET_AUTH_TOKEN) {
    config.HEADERS = { Authorization: `Bearer ${process.env.FAUCET_AUTH_TOKEN}` };
  }
  return new FaucetClient(NODE_URL, FAUCET_URL, config);
}

test("noop", () => {
  // All TS files are compiled by default into the npm package
  // Adding this empty test allows us to:
  // 1. Guarantee that this test library won't get compiled
  // 2. Prevent jest from exploding when it finds a file with no tests in it
});

export const longTestTimeout = 80 * 1000;
