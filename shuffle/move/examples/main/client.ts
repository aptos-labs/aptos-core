// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

// Client for accessing Diem API: https://diem.github.io/diem/diem_api/spec.html

// deno-lint-ignore-file no-explicit-any

import { delay } from "https://deno.land/std@0.114.0/async/delay.ts";

export class Client {
  baseUrl: string

  constructor(baseUrl: string) {
    this.baseUrl = baseUrl;
  }

  async getLedgerInfo(): Promise<any> {
    return await this.fetch(this.url("/"));
  }

  async getTransactions(): Promise<any[]> {
    return await this.fetch(this.url("/transactions"));
  }

  async getTransaction(versionOrHash: number | string): Promise<any> {
    return await this.fetch(this.url(`/transactions/${versionOrHash}`));
  }

  async waitForTransaction(versionOrHash: number | string, timeout?: number): Promise<any> {
    const delayMs = 100;
    const count = (timeout || 5000) / delayMs;

    for (let i = count; i >= 0; i--) {
      const res = await fetch(this.url(`/transactions/${versionOrHash}`));
      const body = await res.json();
      if (res.status === 200) {
        if (body.type !== "pending_transaction") {
          return body;
        }
      } else if (res.status !== 404) {
        throw new Error(JSON.stringify(body))
      }
      if (i > 0) {
        await delay(delayMs);
      }
    }
    throw new Error(`wait for transaction(${versionOrHash}) execution timeout`);
  }

  async getAccountTransactions(addr: string): Promise<any[]> {
    return await this.fetch(this.url(`/accounts/${addr}/transactions`));
  }

  async getAccountResources(addr: string): Promise<any[]> {
    return await this.fetch(this.url(`/accounts/${addr}/resources`));
  }

  async getAccountModules(addr: string): Promise<any[]> {
    return await this.fetch(this.url(`/accounts/${addr}/modules`));
  }

  async submitTransaction(txn: string): Promise<any> {
    return await this.post(this.url("/transactions"), txn);
  }

  async createSigningMessage(txn: string): Promise<any> {
    return await this.post(this.url("/transactions/signing_message"), txn);
  }

  async submitBcsTransaction(txn: string | Uint8Array): Promise<any> {
    return await this.fetch(this.url("/transactions"), {
      method: "POST",
      body: txn,
      headers: {
        "Content-Type": "application/x.diem.signed_transaction+bcs"
      }
    });
  }

  private async post(url: string, body: string | Uint8Array): Promise<any> {
    return await this.fetch(url, {
      method: "POST",
      body: body,
      headers: {
        "Content-Type": "application/json",
      }
    });
  }

  private async fetch(url: string, setting?: any): Promise<any> {
    const res = await fetch(url, setting);
    if (res.status >= 400) {
      throw new Error(JSON.stringify(await res.json()));
    }
    return await res.json();
  }

  private url(tail: string): string {
    return new URL(tail, this.baseUrl).href;
  }
}
