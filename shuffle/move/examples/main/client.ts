// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

// Client for accessing Diem API, please refer to https://diem.github.io/diem/diem_api/spec.html
// for data type and API documentation.

// deno-lint-ignore-file no-explicit-any camelcase

import { delay } from "https://deno.land/std@0.114.0/async/delay.ts";

export class Client {
  baseUrl: string;

  constructor(baseUrl: string) {
    this.baseUrl = baseUrl;
  }

  async getLedgerInfo(): Promise<LedgerInfo> {
    return await this.fetch(this.url("/"));
  }

  async getTransactions(): Promise<OnChainTransaction[]> {
    return await this.fetch(this.url("/transactions"));
  }

  async getTransaction(versionOrHash: number | string): Promise<Transaction> {
    return await this.fetch(this.url(`/transactions/${versionOrHash}`));
  }

  async waitForTransaction(
    versionOrHash: number | string,
    timeout?: number,
  ): Promise<OnChainTransaction> {
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
        throw new Error(JSON.stringify(body));
      }
      if (i > 0) {
        await delay(delayMs);
      }
    }
    throw new Error(`wait for transaction(${versionOrHash}) execution timeout`);
  }

  async getAccount(addr: string): Promise<Account> {
    return await this.fetch(this.url(`/accounts/${addr}`));
  }

  async getAccountTransactions(addr: string): Promise<OnChainTransaction[]> {
    return await this.fetch(this.url(`/accounts/${addr}/transactions`));
  }

  async getAccountResources(addr: string): Promise<any[]> {
    return await this.fetch(this.url(`/accounts/${addr}/resources`));
  }

  async getAccountModules(addr: string): Promise<any[]> {
    return await this.fetch(this.url(`/accounts/${addr}/modules`));
  }

  async getEventsByEventHandle(
    addr: string,
    handleStruct: string,
    fieldName: string,
    start?: number,
    limit?: number
  ): Promise<Event[]> {
    start = start || 0;
    limit = limit || 100;
    const query = `start=${start}&limit=${limit}`
    return await this.fetch(this.url(`/accounts/${addr}/events/${handleStruct}/${fieldName}?${query}`));
  }

  async submitTransaction(
    txn: UserTransactionRequest,
  ): Promise<PendingTransaction> {
    return await this.post(this.url("/transactions"), txn);
  }

  async createSigningMessage(
    txn: SigningMessageRequest,
  ): Promise<SigningMessage> {
    return await this.post(this.url("/transactions/signing_message"), txn);
  }

  async submitBcsTransaction(
    txn: string | Uint8Array,
  ): Promise<PendingTransaction> {
    return await this.fetch(this.url("/transactions"), {
      method: "POST",
      body: txn,
      headers: {
        "Content-Type": "application/x.diem.signed_transaction+bcs",
      },
    });
  }

  private async post(url: string, body: any): Promise<any> {
    return await this.fetch(url, {
      method: "POST",
      body: JSON.stringify(body),
      headers: {
        "Content-Type": "application/json",
      },
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

export interface ScriptFunctionPayload {
  type: "script_function_payload";
  function: string;
  type_arguments: string[];
  arguments: any[];
}

export interface WriteSetPayload {
  type: "write_set_payload";
  write_set: any;
}

export type TransactionPayload = ScriptFunctionPayload | WriteSetPayload;

export interface Ed25519Signature {
  type: "ed25519_signature";
  public_key: string;
  signature: string;
}

export type TransactionSignature = Ed25519Signature;

export interface Event {
  key: string;
  sequence_number: string;
  type: string;
  data: any;
}

export interface SigningMessageRequest {
  sender: string;
  sequence_number: string;
  max_gas_amount: string;
  gas_unit_price: string;
  gas_currency_code: string;
  expiration_timestamp_secs: string;
  payload: TransactionPayload;
}

export interface UserTransactionRequest extends SigningMessageRequest {
  signature: TransactionSignature;
}

export interface PendingTransaction extends UserTransactionRequest {
  type: "pending_transaction";
  hash: string;
}

export interface UserTransaction extends UserTransactionRequest {
  type: "user_transaction";
  hash: string;
  version: string;
  events: Event[];
  state_root_hash: string;
  event_root_hash: string;
  gas_used: string;
  success: boolean;
  vm_status: string;
  timestamp: string;
}

export interface GenesisTransaction {
  type: "genesis_transaction";
  hash: string;
  version: string;
  events: Event[];
  state_root_hash: string;
  event_root_hash: string;
  gas_used: string;
  success: boolean;
  vm_status: string;
  payload: WriteSetPayload;
}

export interface BlockMetadataTransaction {
  type: "block_metadata_transaction";
  hash: string;
  version: string;
  events: Event[];
  state_root_hash: string;
  event_root_hash: string;
  gas_used: string;
  success: boolean;
  vm_status: string;
  id: string;
  round: string;
  previous_block_votes: string[];
  proposer: string;
  timestamp: string;
}

export type OnChainTransaction =
  | UserTransaction
  | GenesisTransaction
  | BlockMetadataTransaction;
export type Transaction = PendingTransaction | OnChainTransaction;

export interface SigningMessage {
  message: string;
}

export interface LedgerInfo {
  chain_id: number;
  ledger_version: string;
  ledger_timestamp: string;
}

export interface Account {
  sequence_number: string;
  authentication_key: string;
}
