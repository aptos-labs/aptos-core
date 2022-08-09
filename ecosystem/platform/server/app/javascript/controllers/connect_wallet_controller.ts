// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Controller } from "./controller";

// Connects to data-controller="connect-wallet"
export default class extends Controller<HTMLFormElement> {
  static targets = ["address"];

  static values = {
    dialog: String,
  };

  declare readonly dialogValue: string;
  declare readonly addressTarget: HTMLInputElement;

  async submit(event: SubmitEvent) {
    event.preventDefault();

    // If the wallet isn't set up, show the "Install Wallet" modal.
    const isInstalled = 'aptos' in window;
    if (!isInstalled) return this.showInstallDialog();

    try {
      // TODO: The connect() promise never resolves if an account isn't set up.
      const account = await window.aptos?.connect();
      if (!account) return this.showInstallDialog();
      const {address} = account;
      this.addressTarget.value = address;
      this.element.submit();
    } catch (err) {
      console.error(err);
      return this.showInstallDialog();
    }
  }

  showInstallDialog() {
    const dialog = document.getElementById(this.dialogValue);
    if (dialog instanceof HTMLDialogElement) {
      dialog.showModal();
    } else {
      console.error('Install Wallet dialog not found.');
    }
  }
}
