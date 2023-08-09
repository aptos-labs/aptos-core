import nacl from "tweetnacl";
import * as bip39 from "@scure/bip39";
import { bytesToHex } from "@noble/hashes/utils";
import { Hex } from "../types";
import { derivePath, Memoize, HexData } from "../utils";
import { AuthenticationKey, Ed25519PublicKey } from "../crypto";

export class Account {
  /**
   * A private key and public key, associated with the given account
   */
  private signingKey: nacl.SignKeyPair;

  /**
   * Address associated with the given account
   */
  private accountAddress: AccountAddress;

  /**
   * Creates new account instance.
   * @param privateKeyBytes  Private key from which account key pair will be generated.
   * If not specified, new key pair is going to be created.
   */
  constructor(privateKeyBytes?: Uint8Array) {
    if (privateKeyBytes) {
      this.signingKey = nacl.sign.keyPair.fromSeed(privateKeyBytes.slice(0, 32));
    } else {
      this.signingKey = nacl.sign.keyPair();
    }
    this.accountAddress = new AccountAddress(this.authKey().toBytes());
  }

  getPublicKey(): Uint8Array {
    return this.signingKey.publicKey;
  }

  getPrivateKey(): Uint8Array {
    return this.signingKey.secretKey;
  }

  public get address(): string {
    return this.accountAddress;
  }

  withPrivateKeyAndAddress(privateKeyBytes: Uint8Array, address: Hex) {
    this.accountAddress = new AccountAddress(HexData.validate(address).toBytes());
    this.signingKey = nacl.sign.keyPair.fromSeed(privateKeyBytes.slice(0, 32));
  }

  /**
   * Check's if the derive path is valid
   */
  static isValidPath(path: string): boolean {
    return /^m\/44'\/637'\/[0-9]+'\/[0-9]+'\/[0-9]+'+$/.test(path);
  }

  /**
   * Creates new account with bip44 path and mnemonics,
   * @param path. (e.g. m/44'/637'/0'/0'/0')
   * Detailed description: {@link https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki}
   * @param mnemonics.
   * @returns AptosAccount
   */
  static fromDerivePath(path: string, mnemonics: string): Account {
    if (!Account.isValidPath(path)) {
      throw new Error("Invalid derivation path");
    }

    const normalizeMnemonics = mnemonics
      .trim()
      .split(/\s+/)
      .map((part) => part.toLowerCase())
      .join(" ");

    const { key } = derivePath(path, bytesToHex(bip39.mnemonicToSeedSync(normalizeMnemonics)));

    return new Account(key);
  }

  sign(data: Hex): HexData {
    const buffer = HexData.validate(data).toBytes();
    const signature = nacl.sign.detached(buffer, this.signingKey.secretKey);
    return HexData.fromBytes(signature);
  }

  verifySignature(message: Hex, signature: Hex): boolean {
    const rawMessage = HexData.validate(message).toBytes();
    const rawSignature = HexData.validate(signature).toBytes();
    return nacl.sign.detached.verify(rawMessage, rawSignature, this.signingKey.publicKey);
  }

  @Memoize()
  authKey(): HexData {
    const pubKey = new Ed25519PublicKey(this.signingKey.publicKey);
    const authKey = AuthenticationKey.fromEd25519PublicKey(pubKey);
    return authKey.derivedAddress();
  }
}
