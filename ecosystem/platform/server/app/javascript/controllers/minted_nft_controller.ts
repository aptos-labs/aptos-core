// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Controller } from "./controller";
import type { FrameElement } from "@hotwired/turbo/dist/types/elements";
import type { Types } from "aptos";

function hexToAscii(hex: string) {
  return hex
    .match(/.{1,2}/g)!
    .map((byte: string) => String.fromCharCode(parseInt(byte, 16)))
    .join("");
}

function decodeMintNumber(mintNumber: string) {
  const ascii = hexToAscii(mintNumber.substring(2));
  return parseInt(ascii, 16);
}

// Connects to data-controller="minted-nft"
export default class extends Controller {
  static values = {
    transactionHash: String,
    transactionVersion: Number,
    apiUrl: String,
  };

  static targets = [
    "transactionFailedError",
    "dateMinted",
    "mintNumber",
    "image",
    "address",
    "transactionLinks",
  ];

  declare readonly transactionFailedErrorTarget: HTMLElement;
  declare readonly dateMintedTarget: HTMLElement;
  declare readonly mintNumberTargets: HTMLElement[];
  declare readonly addressTarget: HTMLElement;
  declare readonly imageTargets: HTMLImageElement[];
  declare readonly transactionLinksTarget: FrameElement;

  declare readonly transactionHashValue: string | null;
  declare readonly transactionVersionValue: number | null;
  declare readonly apiUrlValue: string;

  retries = 0;

  connect() {
    this.fetchNftInfo();
  }

  get transactionUrl() {
    if (this.transactionVersionValue) {
      return [
        this.apiUrlValue,
        "transactions",
        "by_version",
        this.transactionVersionValue,
      ].join("/");
    } else if (this.transactionHashValue) {
      return [
        this.apiUrlValue,
        "transactions",
        "by_hash",
        this.transactionHashValue,
      ].join("/");
    } else {
      throw "unable to determine transaction url";
    }
  }

  fetchNftInfo = async () => {
    const response = await fetch(this.transactionUrl);
    if (!response.ok && ++this.retries <= 1) {
      setTimeout(this.fetchNftInfo, 1000);
      return;
    }

    const transaction: Types.Transaction = await response.json();

    if (!("timestamp" in transaction && "events" in transaction)) return;

    if (!transaction.success) {
      this.transactionFailedErrorTarget.classList.remove("hidden");
      return;
    }

    const urlParams = new URLSearchParams(location.search);
    if (!urlParams.get("v")) {
      urlParams.delete("txn");
      urlParams.set("v", transaction.version);
      const url = new URL(location.href);
      url.search = urlParams.toString();
      this.transactionLinksTarget.src = url.toString();
    }

    const createEvent = transaction.events.find(
      (event) => event.type === "0x3::token::CreateTokenDataEvent"
    );

    if (createEvent == null) return;

    const dateMinted = new Date(parseInt(transaction.timestamp) / 1000);
    const mintNumber = decodeMintNumber(createEvent.data.property_values[0]);
    const imageUrl = createEvent.data.uri;

    this.dateMintedTarget.textContent = dateMinted.toDateString();
    this.mintNumberTargets.forEach((el) => {
      el.textContent = `#${mintNumber}`;
    });
    if ("sender" in transaction) {
      this.addressTarget.textContent =
        transaction.sender.slice(0, 4) + "…" + transaction.sender.slice(-4);
      this.addressTarget.title = transaction.sender;
    }
    this.imageTargets.forEach((el) => {
      el.src = imageUrl;
    });
  };
}
