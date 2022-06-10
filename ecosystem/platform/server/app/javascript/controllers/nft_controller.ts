import { Controller } from "./controller";
import type { FrameElement } from "@hotwired/turbo/dist/types/elements";
import {AptosAccount, AptosClient, FaucetClient, Types} from "aptos";

// TODO: Update with real address / module.
const ADDRESS = "0xb6944e1bcbededcc0be23e33ff9d6d083d2a40bf96f18ac073bf40d9c1750877";
const MODULE = "NFT3";

async function signAndSubmit(client: AptosClient, payload: Types.TransactionPayload, account: AptosAccount) {
  const txnRequest = await client.generateTransaction(account.address(), payload);
  const signedTxn = await client.signTransaction(account, txnRequest);
  const pendingTransaction = await client.submitTransaction(signedTxn);
  await client.waitForTransaction(pendingTransaction.hash);
  return pendingTransaction as Types.OnChainTransaction;
}
async function createFundAccount(faucet: FaucetClient) {
  const account = new AptosAccount();
  await faucet.fundAccount(account.address(), 10_000);
  return account;
}

async function createSubmitMint(client: AptosClient, account: AptosAccount) {
  const payload = { //{ function: string; arguments: string[]; type: string; type_arguments: any[] }
    type: "script_function_payload",
    function: `${ADDRESS}::${MODULE}::mint`,
    type_arguments: [],
    arguments: []
  };
  return await signAndSubmit(client, payload, account);
}

// Connects to data-controller="nft"
export default class extends Controller<FrameElement> {
  static targets = ['form', 'explorerUrl'];

  declare readonly hasFormTarget: boolean;
  declare readonly formTarget: HTMLFormElement;
  declare readonly explorerUrlTarget: HTMLInputElement;

  connect() {
    if (this.hasFormTarget) {
      this.mint();
    }
  }

  async mint() {
    const NODE_URL = "https://fullnode.devnet.aptoslabs.com";
    const FAUCET_URL = "https://faucet.devnet.aptoslabs.com";

    const client = new AptosClient(NODE_URL);
    const faucet = new FaucetClient(NODE_URL, FAUCET_URL);

    let explorerUrl;
    try {
      const account = await createFundAccount(faucet);
      const txn = await createSubmitMint(client, account);
      explorerUrl = `https://explorer.devnet.aptos.dev/txn/${txn.version}`
    } catch {
      // TODO: This is for demonstration purposes only - remove when module is published.
      explorerUrl = 'https://explorer.devnet.aptos.dev/txn/1'
    }

    // Update the explorer_url.
    this.explorerUrlTarget.value = explorerUrl;
    this.formTarget.requestSubmit();
  }
}
