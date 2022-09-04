// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Controller } from "./controller";

const FIELD_NAMES = Object.freeze({
  network: 'wallet[network]',
  walletName: 'wallet[wallet_name]',
  publicKey: 'wallet[public_key]',
  challenge: 'wallet[challenge]',
  signedChallenge: 'wallet[signed_challenge]'
});

// Connects to data-controller="connect-wallet"
export default class extends Controller<HTMLFormElement> {
  get isValid() {
    return Object.values(FIELD_NAMES).every((fieldName) =>
      this.getInput(fieldName).value.length > 0
    );
  }

  get walletName() {
    if ('aptos' in window) {
      return 'petra';
    } else if (false) {
      // TODO: Add more wallet detection logic here.
    } else {
      throw 'Aptos wallet not detected.';
    }
  }

  async getPublicKey() {
    if (this.walletName === 'petra') {
      const {publicKey} = await window.aptos!.connect();
      return publicKey;
    } else if (false) {
      // TODO: Add support for other wallets here.
    } else {
      throw 'Unable to determine public key.'
    }
  }

  async getSignedChallenge() {
    const challenge = this.getInput(FIELD_NAMES.challenge).value;

    if (this.walletName === 'petra') {
      // TODO: Implement real signMessage().
      return `0x${'0'.repeat(128)}`;
    } else if (false) {
      // TODO: Add support for other wallets here.
    } else {
      throw 'Unable to get signed challenge.'
    }
  }

  getInput(fieldName: string): HTMLInputElement {
    const input = this.element[fieldName];
    if (!(input instanceof HTMLInputElement)) {
      throw `input with name ${fieldName} not found.`
    }
    return input;
  }

  async beforeSubmit(event: SubmitEvent) {
    if (this.isValid) {
      // The form is ready for submit. Allow the event to propagate.
      return;
    }

    // Prevent the form submission and get the wallet info.
    event.preventDefault();

    this.getInput(FIELD_NAMES.walletName).value = this.walletName;
    this.getInput(FIELD_NAMES.publicKey).value = await this.getPublicKey();
    this.getInput(FIELD_NAMES.signedChallenge).value = await this.getSignedChallenge();

    this.element.requestSubmit();
  }
}
