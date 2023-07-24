import { AptosAccount, FaucetClient, Network, Provider, HexString } from "aptos";

const NETWORK = Network.DEVNET;
const FAUCET_URL = `https://faucet.${NETWORK}.aptoslabs.com`;
const WIDTH = 16;
const APTOS_COIN_DECIMALS = 8;

function truncate(address: HexString): string {
  return `${address.toString().substring(0, 6)}...${address
    .toString()
    .substring(address.toString().length - 4, address.toString().length)}`;
}

function formatAccountInfo(account: AptosAccount): string {
  const vals: any[] = [
    account.address(),
    account.authKey(),
    HexString.fromUint8Array(account.signingKey.secretKey),
    HexString.fromUint8Array(account.signingKey.publicKey),
  ];

  return vals
    .map((v) => {
      return truncate(v).padEnd(WIDTH);
    })
    .join(" ");
}

(async () => {
  const provider = new Provider(NETWORK);
  const faucetClient = new FaucetClient(provider.aptosClient.nodeUrl, FAUCET_URL);

  // :!:>create_accounts
  const alice = new AptosAccount();
  const bob = new AptosAccount(); // <:!:create_accounts

  await faucetClient.fundAccount(alice.address(), 1 * Math.pow(10, APTOS_COIN_DECIMALS));
  await faucetClient.fundAccount(bob.address(), 1 * Math.pow(10, APTOS_COIN_DECIMALS));

  console.log(
    `\n${"Account".padEnd(WIDTH)} ${"Address".padEnd(WIDTH)} ${"Auth Key".padEnd(WIDTH)} ${"Private Key".padEnd(
      WIDTH,
    )} ${"Public Key".padEnd(WIDTH)}`,
  );
  console.log(`---------------------------------------------------------------------------------`);
  console.log(`${"alice".padEnd(WIDTH)} ${formatAccountInfo(alice)}`);
  console.log(`${"bob".padEnd(WIDTH)} ${formatAccountInfo(bob)}`);
  console.log("\n...rotating...".padStart(WIDTH));

  // Rotate the key!
  // :!:>rotate_key
  const response = await provider.aptosClient.rotateAuthKeyEd25519(alice, bob.signingKey.secretKey); // <:!:rotate_key

  // We must create a new instance of AptosAccount because the private key has changed.
  const aliceNew = new AptosAccount(
    bob.signingKey.secretKey,
    alice.address(), // NOTE: Without this argument, this would be bob, not aliceNew. You must specify the address since the private key matches multiple accounts now
  );

  console.log(`\n${"alice".padEnd(WIDTH)} ${formatAccountInfo(aliceNew)}`);
  console.log(`${"bob".padEnd(WIDTH)} ${formatAccountInfo(bob)}\n`);
})();
