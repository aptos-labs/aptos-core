// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Controller } from "./controller";

// Connects to data-controller="connect-wallet"
export default class extends Controller<HTMLFormElement> {
  static targets = ["ownerKey", "button"];

  static values = {
    dialog: String,
  };

  declare readonly dialogValue: string;
  declare readonly ownerKeyTarget: HTMLInputElement;
  declare readonly buttonTarget: HTMLButtonElement;

  async beforeSubmit(event: SubmitEvent) {
    if (this.ownerKeyTarget.value?.length > 0) {
      // The form is ready for submit. Allow the event to propagate.
      return;
    }

    // Prevent the form submission and get the owner key.
    event.preventDefault();

    if ('aptos' in window) {
      const initialButtonText = this.buttonTarget.textContent;
      try {
        const account = await window.aptos?.connect();
        if (account) {
          // Populate the owner_key input and re-trigger the submit event.
          const {publicKey} = account;
          this.ownerKeyTarget.value = publicKey;
          this.buttonTarget.textContent = '...';
          this.element.requestSubmit();
          return;
        }
      } catch (err) {
        this.buttonTarget.textContent = initialButtonText;
        console.error(err);
      }
    }

    // Something went wrong; show the install dialog.
    const dialog = document.getElementById(this.dialogValue);
    if (dialog instanceof HTMLDialogElement) {
      dialog.showModal();
    } else {
      throw new Error('Install Wallet dialog not found.');
    }
  }
}
