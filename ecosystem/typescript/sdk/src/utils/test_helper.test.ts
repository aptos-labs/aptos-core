export const NODE_URL = process.env.APTOS_NODE_URL!;
export const FAUCET_URL = process.env.APTOS_FAUCET_URL!;
test("noop", () => {
  // All TS files are compiled by default into the npm package
  // Adding this empty test allows us to:
  // 1. Guarantee that this test library won't get compiled
  // 2. Prevent jest from exploding when it finds a file with no tests in it
});
