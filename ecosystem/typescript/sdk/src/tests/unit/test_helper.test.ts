import { FaucetClient } from "../../plugins/faucet_client";
import { OpenAPIConfig } from "../../generated";
import { CustomEndpoints } from "../../utils/api-endpoints";

export const NODE_URL = process.env.APTOS_NODE_URL!;
export const FAUCET_URL = process.env.APTOS_FAUCET_URL!;
export const API_TOKEN = process.env.API_TOKEN!;
export const FAUCET_AUTH_TOKEN = process.env.FAUCET_AUTH_TOKEN!;
export const PROVIDER_LOCAL_NETWORK_CONFIG: CustomEndpoints = { fullnodeUrl: NODE_URL, indexerUrl: NODE_URL };

// account to use for ANS tests, this account matches the one in sdk-release.yaml
export const ANS_OWNER_ADDRESS = "0x585fc9f0f0c54183b039ffc770ca282ebd87307916c215a3e692f2f8e4305e82";
export const ANS_OWNER_PK = "0x37368b46ce665362562c6d1d4ec01a08c8644c488690df5a17e13ba163e20221";

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

export const longTestTimeout = 120 * 1000;
