// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Controller } from "./controller";

interface ClaimDetails {
  wallet_name: string;
  module_address: string;
  message: string;
  signature: string;
}

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
    // TODO: Adjust transaction payload to match module expectations.
    const transaction = {
      type: 'entry_function_payload',
      function: claimDetails.module_address + '::claim_mint',
      arguments: [
        claimDetails.message,
        claimDetails.signature,
      ],
      type_arguments: [],
    };

    if (claimDetails.wallet_name === 'petra') {
      const pendingTransaction = await window.aptos!.signAndSubmitTransaction(transaction);
      // TODO: Do something with the transaction.
    } else if (false) {
      // TODO: Add support for other wallets here.
    } else {
      throw 'Unable to submit transaction.'
    }
  }
}
