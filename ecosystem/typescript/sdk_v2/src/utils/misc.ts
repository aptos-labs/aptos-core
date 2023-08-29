// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { VERSION } from "../version";

export async function sleep(timeMs: number): Promise<null> {
  return new Promise((resolve) => {
    setTimeout(resolve, timeMs);
  });
}

export const DEFAULT_VERSION_PATH_BASE = "/v1";

export function fixNodeUrl(nodeUrl: string): string {
  let out = `${nodeUrl}`;
  if (out.endsWith("/")) {
    out = out.substring(0, out.length - 1);
  }
  if (!out.endsWith(DEFAULT_VERSION_PATH_BASE)) {
    out = `${out}${DEFAULT_VERSION_PATH_BASE}`;
  }
  return out;
}

export const DEFAULT_MAX_GAS_AMOUNT = 200000;
// Transaction expire timestamp
export const DEFAULT_TXN_EXP_SEC_FROM_NOW = 20;
// How long does SDK wait for txn to finish
export const DEFAULT_TXN_TIMEOUT_SEC = 20;
export const APTOS_COIN = "0x1::aptos_coin::AptosCoin";

export const CUSTOM_REQUEST_HEADER = { "x-aptos-client": `aptos-ts-sdk/${VERSION}` };
