// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

export type Nullable<T> = { [P in keyof T]: T[P] | null };

export type AnyObject = { [key: string]: any };

export async function sleep(timeMs: number): Promise<null> {
  return new Promise((resolve) => {
    setTimeout(resolve, timeMs);
  });
}

export const DEFAULT_VERSION_PATH_BASE = "/v1";

export function fixNodeUrl(nodeUrl: string, version = DEFAULT_VERSION_PATH_BASE): string {
  let out = `${nodeUrl}`;
  if (out.endsWith("/")) {
    out = out.substring(0, out.length - 1);
  }
  if (!out.endsWith(version)) {
    out = `${out}${version}`;
  }
  return out;
}
