// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Controller } from "./controller";

interface ClaimDetails {
  wallet_name: string;
  module_address: string;
  message: string;
  signature: string;
}

const fromHexString = (hexString: string) =>
  Array.from(hexString.match(/.{1,2}/g)!
    .map((byte: string) => parseInt(byte, 16)));

// Connects to data-controller="claim-nft"
export default class extends Controller<HTMLAnchorElement> {
  async handleClick(event: Event) {
    event.preventDefault();

    const csrfToken = (document.getElementsByName("csrf-token")[0] as HTMLMetaElement).content;
    const response = await fetch(this.element.href, {
      method: "PUT",
      headers: {
        "X-CSRF-Token": csrfToken,
        "Content-Type": "application/json",
        "Accept": "application/json",
      },
    });

    if (!response.ok) {
      throw 'Unable to retrieve claim details.'
    }

    const json = await response.json();

    if ('error' in json) {
      throw json.error;
    }

    return this.submitTransaction(json as ClaimDetails);
  }

  async submitTransaction(claimDetails: ClaimDetails) {
    // Module expects signature as array of bytes.
    const signature = fromHexString(claimDetails.signature.substring(2));
    const transaction = {
      type: 'entry_function_payload',
      function: claimDetails.module_address + '::claim_mint',
      arguments: [
        claimDetails.message,
        signature,
      ],
      type_arguments: [],
    };

    if (claimDetails.wallet_name === 'petra') {
      const pendingTransaction = await window.aptos!.signAndSubmitTransaction(transaction);
      if ('hash' in pendingTransaction && typeof pendingTransaction.hash === 'string') {
        // TODO: Do something more intelligent with the transaction.
        window.open(`https://explorer.devnet.aptos.dev/txn/${pendingTransaction.hash}?network=testnet`);
        return;
      }
    } else if (false) {
      // TODO: Add support for other wallets here.
    }

    throw 'Unable to submit transaction.'
  }
}
