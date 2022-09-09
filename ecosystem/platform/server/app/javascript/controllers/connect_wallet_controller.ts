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
export default class extends Controller<HTMLFormElement> {
  static targets = ["requiredNetworkError"];

  declare readonly hasRequiredNetworkErrorTarget: boolean;
  declare readonly requiredNetworkErrorTarget: HTMLElement;

  static values = {
    requiredNetwork: String,
    walletPersisted: Boolean,
  };

  declare readonly requiredNetworkValue: string | null;
  declare readonly walletPersistedValue: boolean;

  get walletName() {
    if ("aptos" in window) {
      return "petra";
    } else if ("martian" in window) {
      return "martian";
    } else if (false) {
      // TODO: Add more wallet detection logic here.
    } else {
      throw "Aptos wallet not detected.";
    }
  }

  onPageLoad() {
    const urlParams = new URLSearchParams(location.search);
    if (urlParams.get("wallet") && !this.walletPersistedValue) {
      this.element.requestSubmit();
    }
  }

  async renderErrors() {
    if (this.requiredNetworkValue) {
      const network = await this.getNetwork();
      if (network !== this.requiredNetworkValue) {
        this.requiredNetworkErrorTarget.textContent = `Please set your wallet network to ${this.requiredNetworkValue}. It is currently set to ${network}.`;
        this.requiredNetworkErrorTarget.classList.remove("hidden");
      }
    }
  }

  hideErrors() {
    if (this.hasRequiredNetworkErrorTarget) {
      this.requiredNetworkErrorTarget.classList.add("hidden");
    }
  }

  async getPublicKey() {
    if (this.walletName === "petra") {
      const { publicKey } = await window.aptos!.connect();
      return publicKey;
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
    const input = this.element[fieldName];
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
      this.renderErrors();
      return;
    }

    this.getInput(FIELD_NAMES.walletName).value = this.walletName;
    this.getInput(FIELD_NAMES.network).value = network;
    this.getInput(FIELD_NAMES.publicKey).value = publicKey;
    this.getInput(FIELD_NAMES.signedChallenge).value =
      await this.getSignedChallenge();

    const formData = new FormData(this.element);
    const response = await fetch(this.element.action, {
      method: this.element.method,
      headers: {
        Accept: "application/json",
      },
      body: formData,
    });
    const { created } = await response.json();
    if (created) {
      const urlParams = new URLSearchParams(location.search);
      urlParams.set("wallet", publicKey);
      const url = new URL(location.href);
      url.search = urlParams.toString();

      // Full page load instead of Turbo.visit due to bug with controller not
      // being mounted.
      location.href = url.toString();
    } else {
      console.error("connect wallet failed");
    }
  }
}
