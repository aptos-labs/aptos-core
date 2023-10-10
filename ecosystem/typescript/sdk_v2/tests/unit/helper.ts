// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

export const wallet = {
  address: "0x07968dab936c1bad187c60ce4082f307d030d780e91e694ae03aef16aba73f30",
  mnemonic: "shoot island position soft burden budget tooth cruel issue economy destroy above",
  path: "m/44'/637'/0'/0'/0'",
};

/* eslint-disable max-len */
export const ed25519 = {
  privateKey: "0xc5338cd251c22daa8c9c9cc94f498cc8a5c7e1d2e75287a5dda91096fe64efa5",
  publicKey: "0xde19e5d1880cac87d57484ce9ed2e84cf0f9599f12e7cc3a52e4e7657a763f2c",
  authKey: "0x978c213990c4833df71548df7ce49d54c759d6b6d932de22b24d56060b7af2aa",
  address: "0x978c213990c4833df71548df7ce49d54c759d6b6d932de22b24d56060b7af2aa",
  message: "0x7777",
  signedMessage:
    "0xc5de9e40ac00b371cd83b1c197fa5b665b7449b33cd3cdd305bb78222e06a671a49625ab9aea8a039d4bb70e275768084d62b094bc1b31964f2357b7c1af7e0d",
};

export const multiEd25519PkTestObject = {
  public_keys: [
    "b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a49200",
    "aef3f4a4b8eca1dfc343361bf8e436bd42de9259c04b8314eb8e2054dd6e82ab",
    "8a5762e21ac1cdb3870442c77b4c3af58c7cedb8779d0270e6d4f1e2f7367d74",
  ],
  threshold: 2,
  bytesInStringWithoutPrefix:
    "b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a49200aef3f4a4b8eca1dfc343361bf8e436bd42de9259c04b8314eb8e2054dd6e82ab8a5762e21ac1cdb3870442c77b4c3af58c7cedb8779d0270e6d4f1e2f7367d7402",
};

export const multiEd25519SigTestObject = {
  signatures: [
    "e6f3ba05469b2388492397840183945d4291f0dd3989150de3248e06b4cefe0ddf6180a80a0f04c045ee8f362870cb46918478cd9b56c66076f94f3efd5a8805",
    "2ae0818b7e51b853f1e43dc4c89a1f5fabc9cb256030a908f9872f3eaeb048fb1e2b4ffd5a9d5d1caedd0c8b7d6155ed8071e913536fa5c5a64327b6f2d9a102",
  ],
  bitmap: "c0000000",
  bytesInStringWithoutPrefix:
    "e6f3ba05469b2388492397840183945d4291f0dd3989150de3248e06b4cefe0ddf6180a80a0f04c045ee8f362870cb46918478cd9b56c66076f94f3efd5a88052ae0818b7e51b853f1e43dc4c89a1f5fabc9cb256030a908f9872f3eaeb048fb1e2b4ffd5a9d5d1caedd0c8b7d6155ed8071e913536fa5c5a64327b6f2d9a102c0000000",
};

export const secp256k1TestObject = {
  privateKey: "0xd107155adf816a0a94c6db3c9489c13ad8a1eda7ada2e558ba3bfa47c020347e",
  publicKey:
    "0x04acdd16651b839c24665b7e2033b55225f384554949fef46c397b5275f37f6ee95554d70fb5d9f93c5831ebf695c7206e7477ce708f03ae9bb2862dc6c9e033ea",
  address: "0x44b9b90a0bd6a691a20cb06148f10ec9c21da63bb5df345ae38507e0c3c2f897",
  authKey: "0x44b9b90a0bd6a691a20cb06148f10ec9c21da63bb5df345ae38507e0c3c2f897",
  messageEncoded: "68656c6c6f20776f726c64", // "hello world"
  signatureHex:
    "0x3eda29841168c902b154ac12dfb0f8775ece1b95315b227ede64cbd715abac665aa8c8df5b108b0d4918bb88ea58c892972af375a71761a7e590655ff5de3859",
};
