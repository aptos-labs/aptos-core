// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Controller } from "./controller";
import type { Types } from "aptos";

// Connects to data-controller="minted-nft"
export default class extends Controller {
  static values = {
    transactionHash: String,
    apiUrl: String,
  };

  static targets = ["transactionFailedError", "dateMinted", "mintNumber", "image"];

  declare readonly transactionFailedErrorTarget: HTMLElement;
  declare readonly dateMintedTarget: HTMLElement;
  declare readonly mintNumberTarget: HTMLElement;
  declare readonly imageTarget: HTMLImageElement;

  declare readonly transactionHashValue: string;
  declare readonly apiUrlValue: string;

  connect() {
    this.fetchNftInfo();
  }

  async fetchNftInfo() {
    const transactionUrl = [
      this.apiUrlValue,
      'transactions',
      'by_hash',
      this.transactionHashValue,
    ].join('/');
    const response = await fetch(transactionUrl);
    const transaction: Types.OnChainTransaction = await response.json();

    if (!('timestamp' in transaction && 'events' in transaction)) return;

    if (!transaction.success) {
      this.transactionFailedErrorTarget.classList.remove('hidden');
      return;
    }

    const createEvent = transaction.events.find(event =>
      event.type === '0x3::token::CreateTokenDataEvent');

    if (createEvent == null) return;

    const dateMinted = new Date(parseInt(transaction.timestamp) / 1000);
    const mintNumber = parseInt(createEvent.data.property_values[0], 16);
    const imageUrl = createEvent.data.uri;

    this.dateMintedTarget.textContent = dateMinted.toDateString();
    this.mintNumberTarget.textContent = `#${mintNumber}`;
    this.imageTarget.src = imageUrl;
  }
}
