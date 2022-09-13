// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { MaybeHexString } from 'aptos';

/**
 * Collapse hex string and keep only desired amount of digits.
 * Collapsing is done in the middle of the string, as head and tail are
 * useful for quick glance comparison
 */
export function collapseHexString(
  hex: MaybeHexString,
  keepDigits: number = 10,
  isBalanced?: boolean,
): string {
  const hexStr = hex.toString();
  const digits = hexStr.split('x')[1];

  if (digits.length <= keepDigits) {
    return hexStr;
  }

  const keepDigitsLeft = (isBalanced) ? Math.ceil(keepDigits / 2 - 2) : Math.ceil(keepDigits / 2);
  const keepDigitsRight = Math.floor(keepDigits / 2);
  return `0x${digits.slice(0, keepDigitsLeft)}..${digits.slice(-keepDigitsRight)}`;
}

export default collapseHexString;
