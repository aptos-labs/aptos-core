// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

export function isAddressValid(address?: string) {
  return address
    ? (address.length >= 64 && address.length <= 68)
    : false;
}

export function formatAddress(address?: string) {
  return (address && address.startsWith('0x')) ? address : `0x${address}`;
}
