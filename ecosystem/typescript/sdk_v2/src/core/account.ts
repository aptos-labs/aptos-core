import nacl from "tweetnacl";
import * as bip39 from "@scure/bip39";
import { bytesToHex } from "@noble/hashes/utils";
import { derivePath, Memoize } from "../utils";

export class Account {
  /**
   * A private key and public key, associated with the given account
   */

  /**
   * Address associated with the given account
   */

  /**
   * Creates new account instance.
   * @param privateKeyBytes  Private key from which account key pair will be generated.
   * If not specified, new key pair is going to be created.
   */
  constructor(private signingKey: nacl.SignKeyPair, private accountAddress: AccountAddress) {}

  public get publicKey(): Uint8Array {
    return this.signingKey.publicKey;
  }

  public get privateKey(): Uint8Array {
    return this.signingKey.secretKey;
  }

  public get address(): string {
    return this.accountAddress;
  }

  static generateFromPrivateKey(privateKeyBytes: Uint8Array) {
    const signingKey = nacl.sign.keyPair.fromSeed(privateKeyBytes.slice(0, 32));
    const accountAddress = new AccountAddress(Account.authKey(signingKey.publicKey).toBytes());

    return new Account(signingKey, accountAddress);
  }

  static generateFromPrivateKeyAndAddress(privateKeyBytes: Uint8Array, address: AccountAddress) {
    const accountAddress = new AccountAddress(Hex.validate(address).toBytes());
    const signingKey = nacl.sign.keyPair.fromSeed(privateKeyBytes.slice(0, 32));

    return new Account(signingKey, accountAddress);
  }

  static generate() {
    const signingKey = nacl.sign.keyPair();
    const accountAddress = new AccountAddress(Account.authKey(signingKey.publicKey).toBytes());

    return new Account(signingKey, accountAddress);
  }

  @Memoize()
  static authKey(publicKey: Uint8Array): Hex {
    const pubKey = new Ed25519PublicKey(publicKey);
    const authKey = AuthenticationKey.fromEd25519PublicKey(pubKey);
    return authKey.derivedAddress();
  }

  /**
   * Creates new account with bip44 path and mnemonics,
   * @param path. (e.g. m/44'/637'/0'/0'/0')
   * Detailed description: {@link https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki}
   * @param mnemonics.
   * @returns AptosAccount
   */
  static generateFromDerivePath(path: string, mnemonics: string): Account {
    if (!Account.isValidPath(path)) {
      throw new Error("Invalid derivation path");
    }

    const normalizeMnemonics = mnemonics
      .trim()
      .split(/\s+/)
      .map((part) => part.toLowerCase())
      .join(" ");

    const { key } = derivePath(path, bytesToHex(bip39.mnemonicToSeedSync(normalizeMnemonics)));

    const signingKey = nacl.sign.keyPair.fromSeed(key.slice(0, 32));
    const accountAddress = new AccountAddress(Account.authKey(signingKey.publicKey).toBytes());

    return new Account(signingKey, accountAddress);
  }

  /**
   * Check's if the derive path is valid
   */
  static isValidPath(path: string): boolean {
    return /^m\/44'\/637'\/[0-9]+'\/[0-9]+'\/[0-9]+'+$/.test(path);
  }

  sign(data: Hex): Hex {
    const buffer = Hex.validate(data).toBytes();
    const signature = nacl.sign.detached(buffer, this.signingKey.secretKey);
    return Hex.fromBytes(signature);
  }

  verifySignature(message: Hex, signature: Hex): boolean {
    const rawMessage = Hex.validate(message).toBytes();
    const rawSignature = Hex.validate(signature).toBytes();
    return nacl.sign.detached.verify(rawMessage, rawSignature, this.signingKey.publicKey);
  }
}
