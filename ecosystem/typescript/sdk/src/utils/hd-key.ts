import nacl from "tweetnacl";
import { hmac, sha512 } from "hash.js";
import { bytesToHex } from "../bytes_to_hex";

type Hex = string;
type Path = string;

type Keys = {
  key: Hex;
  chainCode: Hex;
};

const pathRegex = /^m(\/[0-9]+')+$/;

const replaceDerive = (val: string): string => val.replace("'", "");

const HMAC_KEY = "656432353531392073656564"; // ed25519 seed
const HARDENED_OFFSET = 0x80000000;

export const getMasterKeyFromSeed = (seed: Hex): Keys => {
  // @ts-ignore
  const h = hmac(sha512, HMAC_KEY, "hex");
  const I = h.update(seed, "hex").digest("hex");
  const IL = I.slice(0, 64);
  const IR = I.slice(64);
  return {
    key: IL,
    chainCode: IR,
  };
};

export const CKDPriv = ({ key, chainCode }: Keys, index: number): Keys => {
  const indexBuffer = Buffer.allocUnsafe(4);
  indexBuffer.writeUInt32BE(index, 0);

  const buffer = new ArrayBuffer(4);
  new DataView(buffer).setUint32(0, index);
  const indexBytes = new Uint8Array(buffer);
  const zero = new Uint8Array([0]);
  const data = bytesToHex(zero) + key + bytesToHex(indexBytes);

  // @ts-ignore
  const I = hmac(sha512, chainCode, "hex").update(data, "hex").digest("hex");
  const IL = I.slice(0, 64);
  const IR = I.slice(64);
  return {
    key: IL,
    chainCode: IR,
  };
};

export const getPublicKey = (privateKey: Uint8Array, withZeroByte = true): Uint8Array => {
  const keyPair = nacl.sign.keyPair.fromSeed(privateKey);
  const signPk = keyPair.secretKey.subarray(32);
  const zero = new Uint8Array([0]);
  return withZeroByte ? new Uint8Array([...zero, ...signPk]) : signPk;
};

export const isValidPath = (path: string): boolean => {
  if (!pathRegex.test(path)) {
    return false;
  }
  return !path
    .split("/")
    .slice(1)
    .map(replaceDerive)
    .some(Number.isNaN as any);
};

export const derivePath = (path: Path, seed: Hex, offset = HARDENED_OFFSET): Keys => {
  if (!isValidPath(path)) {
    throw new Error("Invalid derivation path");
  }

  const { key, chainCode } = getMasterKeyFromSeed(seed);
  const segments = path
    .split("/")
    .slice(1)
    .map(replaceDerive)
    .map((el) => parseInt(el, 10));

  return segments.reduce((parentKeys, segment) => CKDPriv(parentKeys, segment + offset), { key, chainCode });
};
