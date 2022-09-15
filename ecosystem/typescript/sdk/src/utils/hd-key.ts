import nacl from "tweetnacl";
import { hmac } from "@noble/hashes/hmac";
import { sha512 } from "@noble/hashes/sha512";
import { hexToBytes } from "@noble/hashes/utils";

type Hex = string;
type Path = string;

type Keys = {
  key: Uint8Array;
  chainCode: Uint8Array;
};

const pathRegex = /^m(\/[0-9]+')+$/;

const replaceDerive = (val: string): string => val.replace("'", "");

const HMAC_KEY = "ed25519 seed";
const HARDENED_OFFSET = 0x80000000;

export const getMasterKeyFromSeed = (seed: Hex): Keys => {
  const h = hmac.create(sha512, HMAC_KEY);
  const I = h.update(hexToBytes(seed)).digest();
  const IL = I.slice(0, 32);
  const IR = I.slice(32);
  return {
    key: IL,
    chainCode: IR,
  };
};

export const CKDPriv = ({ key, chainCode }: Keys, index: number): Keys => {
  const buffer = new ArrayBuffer(4);
  new DataView(buffer).setUint32(0, index);
  const indexBytes = new Uint8Array(buffer);
  const zero = new Uint8Array([0]);
  const data = new Uint8Array([...zero, ...key, ...indexBytes]);

  const I = hmac.create(sha512, chainCode).update(data).digest();
  const IL = I.slice(0, 32);
  const IR = I.slice(32);
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
