// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosAccount, AptosAccountObject } from "./aptos_account";

const aptosAccountObject: AptosAccountObject = {
  address: "0x978c213990c4833df71548df7ce49d54c759d6b6d932de22b24d56060b7af2aa",
  privateKeyHex:
    // eslint-disable-next-line max-len
    "0xc5338cd251c22daa8c9c9cc94f498cc8a5c7e1d2e75287a5dda91096fe64efa5de19e5d1880cac87d57484ce9ed2e84cf0f9599f12e7cc3a52e4e7657a763f2c",
  publicKeyHex: "0xde19e5d1880cac87d57484ce9ed2e84cf0f9599f12e7cc3a52e4e7657a763f2c",
};

const mnemonic = "shoot island position soft burden budget tooth cruel issue economy destroy above";

test("generates random accounts", () => {
  const a1 = new AptosAccount();
  const a2 = new AptosAccount();
  expect(a1.authKey()).not.toBe(a2.authKey());
  expect(a1.address().hex()).not.toBe(a2.address().hex());
});

test("generates derive path accounts", () => {
  const address = "0x07968dab936c1bad187c60ce4082f307d030d780e91e694ae03aef16aba73f30";
  const a1 = AptosAccount.fromDerivePath("m/44'/637'/0'/0'/0'", mnemonic);
  expect(a1.address().hex()).toBe(address);
});

test("generates derive path accounts", () => {
  expect(() => {
    AptosAccount.fromDerivePath("", mnemonic);
  }).toThrow(new Error("Invalid derivation path"));
});

test("accepts custom address", () => {
  const address = "0x777";
  const a1 = new AptosAccount(undefined, address);
  expect(a1.address().hex()).toBe(address);
});

test("Deserializes from AptosAccountObject", () => {
  const a1 = AptosAccount.fromAptosAccountObject(aptosAccountObject);
  expect(a1.address().hex()).toBe(aptosAccountObject.address);
  expect(a1.pubKey().hex()).toBe(aptosAccountObject.publicKeyHex);
});

test("Deserializes from AptosAccountObject without address", () => {
  const privateKeyObject = { privateKeyHex: aptosAccountObject.privateKeyHex };
  const a1 = AptosAccount.fromAptosAccountObject(privateKeyObject);
  expect(a1.address().hex()).toBe(aptosAccountObject.address);
  expect(a1.pubKey().hex()).toBe(aptosAccountObject.publicKeyHex);
});

test("Serializes/Deserializes", () => {
  const a1 = new AptosAccount();
  const a2 = AptosAccount.fromAptosAccountObject(a1.toPrivateKeyObject());
  expect(a1.authKey().hex()).toBe(a2.authKey().hex());
  expect(a1.address().hex()).toBe(a2.address().hex());
});

test("Signs Strings", () => {
  const a1 = AptosAccount.fromAptosAccountObject(aptosAccountObject);
  expect(a1.signHexString("0x7777").hex()).toBe(
    // eslint-disable-next-line max-len
    "0xc5de9e40ac00b371cd83b1c197fa5b665b7449b33cd3cdd305bb78222e06a671a49625ab9aea8a039d4bb70e275768084d62b094bc1b31964f2357b7c1af7e0d",
  );
});
