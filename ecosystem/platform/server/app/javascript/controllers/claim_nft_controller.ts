// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Controller } from "./controller";
import type { Types } from "aptos";
import * as Sentry from "@sentry/browser";

interface ClaimDetails {
  wallet_name: string;
  module_address: string;
  message: string;
  signature: string;
}

const fromHexString = (hexString: string) =>
  Array.from(
    hexString.match(/.{1,2}/g)!.map((byte: string) => parseInt(byte, 16))
  );

// Connects to data-controller="claim-nft"
export default class extends Controller<HTMLAnchorElement> {
  static targets = ["form", "transactionFailedError"];

  declare readonly formTarget: HTMLFormElement;
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
    const accountTransactionsUrl = [
      this.apiUrlValue,
      "accounts",
      this.addressValue,
      "transactions",
    ].join("/");
    const response = await fetch(accountTransactionsUrl);
    if (!response.ok) return;
    const transactions: Types.Transaction[] = await response.json();
    const mintTransaction = transactions.find(
      (transaction) =>
        "success" in transaction &&
        transaction.success &&
        "payload" in transaction &&
        "function" in transaction.payload &&
        transaction.payload.function === this.mintFunctionName
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

  async handleSubmit(event: Event) {
    event.preventDefault();
    this.transactionFailedErrorTarget.classList.add("hidden");

    const formData = new FormData(this.formTarget);
    const response = await fetch(this.formTarget.action, {
      method: this.formTarget.method,
      headers: {
        Accept: "application/json",
      },
      body: formData,
    });

    if (!response.ok) {
      if (response.redirected) {
        location.href = response.url;
        return;
      }
      throw "Unable to retrieve claim details.";
    }

    const json = await response.json();

    if ("error" in json) {
      if (json.error === "account_not_found") {
        this.transactionFailedErrorTarget.classList.remove("hidden");
      } else if (json.error === "captcha_invalid") {
        const urlParams = new URLSearchParams(location.search);
        urlParams.set("captcha2", "1");
        const url = new URL(location.href);
        url.search = urlParams.toString();
        location.href = url.toString();
      }
      const error = new Error(json.error);
      Sentry.captureException(error);
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
        const pendingTransaction = await window.aptos!.signAndSubmitTransaction(
          transaction
        );
        if (
          "hash" in pendingTransaction &&
          typeof pendingTransaction.hash === "string"
        ) {
          return this.redirectToTransaction(pendingTransaction.hash);
        }
      } catch (error: any) {
        if (error.name === "Unauthorized") {
          // if unauthorized, we need to connect to wallet
          const url = new URL(location.href);
          url.search = ``;
          location.href = url.toString();
        } else {
          Sentry.captureException(error);
        }
      }
    } else if (claimDetails.wallet_name === "martian") {
      try {
        const { address } = await window.martian!.connect();
        const txnHash = await window.martian!.generateSignAndSubmitTransaction(
          address,
          transaction
        );
        return this.redirectToTransaction(txnHash);
      } catch (error) {
        Sentry.captureException(error);
      }
    } else if (false) {
      // TODO: Add support for other wallets here.
    }

    this.transactionFailedErrorTarget.classList.remove("hidden");
  }
}
