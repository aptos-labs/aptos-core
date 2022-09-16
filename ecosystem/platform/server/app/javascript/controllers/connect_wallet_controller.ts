// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Controller } from "./controller";

const FIELD_NAMES = Object.freeze({
  network: "wallet[network]",
  walletName: "wallet[wallet_name]",
  publicKey: "wallet[public_key]",
  challenge: "wallet[challenge]",
  signedChallenge: "wallet[signed_challenge]",
});

// Connects to data-controller="connect-wallet"
export default class extends Controller<HTMLElement> {
  static targets = ["form", "errors"];

  declare readonly formTarget: HTMLFormElement;
  declare readonly errorsTarget: HTMLElement;

  static values = {
    requiredNetwork: String,
  };

  declare readonly requiredNetworkValue: string | null;

  get walletName() {
    return this.getInput(FIELD_NAMES.walletName).value;
  }

  selectWallet(event: Event) {
    if (!(event.currentTarget instanceof HTMLButtonElement)) return;
    const walletName = event.currentTarget.dataset.wallet;
    if (!walletName) return;
    this.element.querySelector("dialog")?.close();
    this.getInput(FIELD_NAMES.walletName).value = walletName;
    this.formTarget.requestSubmit();
  }

  async renderErrors(errors: string[]) {
    if (this.requiredNetworkValue) {
      const network = await this.getNetwork();
      if (network !== this.requiredNetworkValue) {
        errors.push(
          `Please set your wallet network to ${this.requiredNetworkValue}. It is currently set to ${network}.`
        );
      }
    }
    if (errors.length > 0) {
      this.errorsTarget.classList.remove("hidden");
      const ul = document.createElement("ul");
      for (const error of errors) {
        const li = document.createElement("li");
        li.textContent = error;
        ul.appendChild(li);
      }
      this.errorsTarget.querySelector("ul")?.replaceWith(ul);
    }
  }

  hideErrors() {
    this.errorsTarget.classList.add("hidden");
  }

  async getPublicKey() {
    if (this.walletName === "petra") {
      if (window.aptos) {
        const { publicKey } = await window.aptos.connect();
        return publicKey;
      } else {
        window.open(
          "https://chrome.google.com/webstore/detail/petra-aptos-wallet/ejjladinnckdgjemekebdpeokbikhfci",
          "_blank"
        );
        throw "Petra wallet not installed. Install from the Chrome Web Store and refresh the page.";
      }
    } else if (this.walletName === "martian") {
      const { publicKey } = await window.martian!.connect();
      return publicKey;
    } else if (false) {
      // TODO: Add support for other wallets here.
    } else {
      throw "Unable to determine public key.";
    }
  }

  async getNetwork() {
    if (this.walletName === "petra") {
      const network = await window.aptos!.network();
      return network.toLowerCase();
    } else if (this.walletName === "martian") {
      const network = await window.martian!.network();
      return network.toLowerCase();
    } else if (false) {
      // TODO: Add support for other wallets here.
    } else {
      throw "Unable to determine public key.";
    }
  }

  async getSignedChallenge() {
    const challenge = this.getInput(FIELD_NAMES.challenge).value;

    if (this.walletName === "petra") {
      const response = await window.aptos!.signMessage({
        message: "verify_wallet",
        nonce: challenge,
      });
      if ("signature" in response && typeof response.signature === "string") {
        return "0x" + response.signature.slice(0, 128);
      }
    } else if (this.walletName === "martian") {
      const response = await window.martian!.signMessage({
        message: "verify_wallet",
        nonce: challenge,
      });
      if ("signature" in response && typeof response.signature === "string") {
        return response.signature;
      }
    } else if (false) {
      // TODO: Add support for other wallets here.
    }

    throw "Unable to get signed challenge.";
  }

  getInput(fieldName: string): HTMLInputElement {
    const input = this.formTarget[fieldName];
    if (!(input instanceof HTMLInputElement)) {
      throw `input with name ${fieldName} not found.`;
    }
    return input;
  }

  async beforeSubmit(event: SubmitEvent) {
    // Prevent the form submission and get the wallet info.
    event.preventDefault();
    this.hideErrors();

    const publicKey = await this.getPublicKey();
    const network = await this.getNetwork();
    if (this.requiredNetworkValue && network !== this.requiredNetworkValue) {
      this.renderErrors([]);
      return;
    }

    this.getInput(FIELD_NAMES.walletName).value = this.walletName;
    this.getInput(FIELD_NAMES.network).value = network;
    this.getInput(FIELD_NAMES.publicKey).value = publicKey;
    this.getInput(FIELD_NAMES.signedChallenge).value =
      await this.getSignedChallenge();

    const formData = new FormData(this.formTarget);
    const response = await fetch(this.formTarget.action, {
      method: this.formTarget.method,
      headers: {
        Accept: "application/json",
      },
      body: formData,
    });
    const json = await response.json();
    if (json.created) {
      const urlParams = new URLSearchParams(location.search);
      urlParams.set("wallet", publicKey);
      const url = new URL(location.href);
      url.search = urlParams.toString();

      // Full page load instead of Turbo.visit due to bug with controller not
      // being mounted.
      location.href = url.toString();
    } else if ("errors" in json) {
      this.renderErrors(json.errors);
    } else {
      console.error("connect wallet failed");
    }
  }
}
