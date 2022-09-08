// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Controller } from "./controller";
import type { Types } from "aptos";

interface ClaimDetails {
  wallet_name: string;
  module_address: string;
  message: string;
  signature: string;
}

const fromHexString = (hexString: string) =>
  Array.from(hexString.match(/.{1,2}/g)!.map((byte: string) => parseInt(byte, 16)));

// Connects to data-controller="claim-nft"
export default class extends Controller<HTMLAnchorElement> {
  static targets = ["transactionFailedError"];

  declare readonly transactionFailedErrorTarget: HTMLElement;

  static values = {
    address: String,
    network: String,
    apiUrl: String,
    moduleAddress: String,
  };

  declare readonly addressValue: string;
  declare readonly networkValue: string;
  declare readonly apiUrlValue: string;
  declare readonly moduleAddressValue: string;

  get mintFunctionName() {
    return this.moduleAddressValue.replace(/0x0+/, "0x") + "::claim_mint";
  }

  connect() {
    this.redirectIfMinted();
  }

  async redirectIfMinted() {
    const accountTransactionsUrl = [this.apiUrlValue, "accounts", this.addressValue, "transactions"].join("/");
    const response = await fetch(accountTransactionsUrl);
    if (!response.ok) return;
    const transactions: Types.OnChainTransaction[] = await response.json();
    const mintTransaction = transactions.find(
      (transaction) =>
        transaction.success &&
        "payload" in transaction &&
        "function" in transaction.payload &&
        transaction.payload.function === this.mintFunctionName,
    );
    if (mintTransaction) {
      this.redirectToTransaction(mintTransaction.hash);
    }
  }

  redirectToTransaction(hash: string) {
    const url = new URL(location.href);
    url.search = `?txn=${hash}`;

    // Full page load instead of Turbo.visit due to bug with controller not
    // being mounted.
    location.href = url.toString();
  }

  async handleClick(event: Event) {
    event.preventDefault();
    this.transactionFailedErrorTarget.classList.add("hidden");

    const csrfToken = (document.getElementsByName("csrf-token")[0] as HTMLMetaElement).content;
    const response = await fetch(this.element.querySelector("a")!.href, {
      method: "PUT",
      headers: {
        "X-CSRF-Token": csrfToken,
        "Content-Type": "application/json",
        Accept: "application/json",
      },
    });

    if (!response.ok) {
      throw "Unable to retrieve claim details.";
    }

    const json = await response.json();

    if ("error" in json) {
      this.transactionFailedErrorTarget.classList.remove("hidden");
      console.error(json.error);
      return;
    }

    return this.submitTransaction(json as ClaimDetails);
  }

  async submitTransaction(claimDetails: ClaimDetails) {
    // Module expects signature as array of bytes.
    const signature = fromHexString(claimDetails.signature.substring(2));
    const transaction = {
      type: "entry_function_payload",
      function: this.mintFunctionName,
      arguments: [claimDetails.message, signature],
      type_arguments: [],
    };
    this.transactionFailedErrorTarget.classList.add("hidden");

    if (claimDetails.wallet_name === "petra") {
      try {
        const pendingTransaction = await window.aptos!.signAndSubmitTransaction(transaction);
        if ("hash" in pendingTransaction && typeof pendingTransaction.hash === "string") {
          return this.redirectToTransaction(pendingTransaction.hash);
        }
      } catch (error) {
        console.error(error);
      }
    } else if (claimDetails.wallet_name === "martian") {
      try {
        const txnHash = await window.martian!.signAndSubmitTransaction(transaction);
        return this.redirectToTransaction(txnHash);
      } catch (error) {
        console.error(error);
      }
    } else if (false) {
      // TODO: Add support for other wallets here.
    }

    this.transactionFailedErrorTarget.classList.remove("hidden");
  }
}
