// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Controller } from "./controller";
import type { Types } from "aptos";

function hexToAscii(hex: string) {
  return hex.match(/.{1,2}/g)!
    .map((byte: string) => String.fromCharCode(parseInt(byte, 16)))
    .join('');
}

function decodeMintNumber(mintNumber: string) {
  const ascii = hexToAscii(mintNumber.substring(2));
  return parseInt(ascii, 16);
}

// Connects to data-controller="minted-nft"
export default class extends Controller {
  static values = {
    transactionHash: String,
    apiUrl: String,
  };

  static targets = ["transactionFailedError", "dateMinted", "mintNumber", "image", "address"];

  declare readonly transactionFailedErrorTarget: HTMLElement;
  declare readonly dateMintedTarget: HTMLElement;
  declare readonly mintNumberTarget: HTMLElement;
  declare readonly addressTarget: HTMLElement;
  declare readonly imageTarget: HTMLImageElement;

  declare readonly transactionHashValue: string;
  declare readonly apiUrlValue: string;

  retries = 0;

  connect() {
    this.fetchNftInfo();
  }

  fetchNftInfo = async () => {
    const transactionUrl = [
      this.apiUrlValue,
      'transactions',
      'by_hash',
      this.transactionHashValue,
    ].join('/');
    const response = await fetch(transactionUrl);
    if (!response.ok && ++this.retries <= 1) {
      return setTimeout(this.fetchNftInfo, 1000);
    }

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
    const mintNumber = decodeMintNumber(createEvent.data.property_values[0]);
    const imageUrl = createEvent.data.uri;

    this.dateMintedTarget.textContent = dateMinted.toDateString();
    this.mintNumberTarget.textContent = `#${mintNumber}`;
    this.addressTarget.textContent = transaction.sender.slice(0, 4) + "…" + transaction.sender.slice(-4);
    this.addressTarget.title = transaction.sender;
    this.imageTarget.src = imageUrl;
  }
}
