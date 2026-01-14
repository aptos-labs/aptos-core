// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
import type { Fp2 } from '@noble/curves/abstract/tower.js';
import type { WeierstrassPoint, WeierstrassPointCons } from '@noble/curves/abstract/weierstrass.js';
import { bn254 } from '@noble/curves/bn254.js';
import { leBytesToBigint, bigintToLEBytesFq, bigintToLEBytesFr } from './fieldSerialization.ts';

const SWFlag = {
  PointAtInfinity : 1 << 6,
  YIsNegative : 1 << 7,
  YIsPositive : 0,
} as const;

export const G1_SIZE = 32;
export const G2_SIZE = 64;

function fp2LessThanEq(p0: Fp2, p1: Fp2): boolean {
  if (p0.c1 > p1.c1) {
    return false;
  } else if (p0.c1 < p1.c1) {
    return true;
  } else {
    if (p0.c0 > p1.c0) {
      return false;
    } else {
      return true;
    }
  }
}

export function weierstrassEquation<T>(x: T, p: WeierstrassPointCons<T>): T {
  const x2 = p.Fp.sqr(x); // x * x
  const x3 = p.Fp.mul(x2, x); // x² * x
  return p.Fp.add(p.Fp.add(x3, p.Fp.mul(x, p.CURVE().a)), p.CURVE().b); // x³ + a * x + b
}

function ensureNegative(x: bigint, p: WeierstrassPointCons<bigint>) {
    return x <= p.Fp.neg(x) ? p.Fp.neg(x) : x;
}
function ensurePositive(x: bigint, p: WeierstrassPointCons<bigint>) {
    return x <= p.Fp.neg(x) ? x : p.Fp.neg(x);
}

function ensureNegativeFp2(x: Fp2, p: WeierstrassPointCons<Fp2>) {
    return fp2LessThanEq(x, p.Fp.neg(x)) ? p.Fp.neg(x) : x;
}
function ensurePositiveFp2(x: Fp2, p: WeierstrassPointCons<Fp2>) {
    return fp2LessThanEq(x, p.Fp.neg(x)) ? x : p.Fp.neg(x);
}

export function g1ToBytes(p: WeierstrassPoint<bigint>): Uint8Array {
  if (p.is0()) {
    let bytes = new Uint8Array(32);
    bytes[31] |= SWFlag.PointAtInfinity;
    return bytes;
  } else {
    var affine = p.toAffine();

    let flag = affine.y <= bn254.G1.Point.Fp.neg(affine.y) ? SWFlag.YIsPositive : SWFlag.YIsNegative;

    let bytes = bigintToLEBytesFq(affine.x);
    bytes[31] |= flag;
    return bytes;
  }
}


export function bytesToG1(bytes: Uint8Array): WeierstrassPoint<bigint> {
  if (bytes.length != 32) {
    throw new Error("G1 byte representation must be 32 bytes");
  } else {
    // save the most significant two bits of `bytes`
    let flag = bytes[31] & 0xC0;
    // zero out those two bits
    bytes[31] ^= flag;
    if (flag == SWFlag.PointAtInfinity) {
      return bn254.G1.Point.ZERO;
    } else {
      let x = leBytesToBigint(bytes);
      if (!bn254.G1.Point.Fp.isValid(x)) throw new Error('bad point: is not on curve, wrong x');
      let y_squared = weierstrassEquation(x, bn254.G1.Point);
      let y_or_neg_y;
      try {
      y_or_neg_y = bn254.G1.Point.Fp.sqrt(y_squared);
      } catch (sqrtError) {
        const err = sqrtError instanceof Error ? ': ' + sqrtError.message : '';
        throw new Error('bad point: is not on curve, sqrt error' + err);
      }
      let y = flag == SWFlag.YIsPositive ? 
        ensurePositive(y_or_neg_y, bn254.G1.Point) :
        ensureNegative(y_or_neg_y, bn254.G1.Point);
      return new bn254.G1.Point(x, y, 1n);
    }
  }
}


export function g2ToBytes(p: WeierstrassPoint<Fp2>): Uint8Array {
  if (p.is0()) {
    let bytes = new Uint8Array(64);
    bytes[63] |= SWFlag.PointAtInfinity;
    return bytes;
  } else {
    var affine = p.toAffine();

    let flag = fp2LessThanEq(affine.y, bn254.G2.Point.Fp.neg(affine.y)) ? SWFlag.YIsPositive : SWFlag.YIsNegative;

    let c0_bytes = bigintToLEBytesFq(affine.x.c0);
    let c1_bytes = bigintToLEBytesFq(affine.x.c1);
    c1_bytes[31] |= flag;

    let bytes = new Uint8Array(64);
    bytes.set(c0_bytes);
    bytes.set(c1_bytes, 32);

    return bytes;
  }
}

export function bytesToG2(bytes: Uint8Array): WeierstrassPoint<Fp2> {
  if (bytes.length != 64) {
    throw new Error("G2 byte representation must be 64 bytes");
  } else {
    // save the most significant two bits of `bytes`
    let flag = bytes[63] & 0xC0;
    // zero out those two bits
    bytes[63] ^= flag;
    if (flag == SWFlag.PointAtInfinity) {
      return bn254.G2.Point.ZERO;
    } else {
      let c0 = leBytesToBigint(bytes.slice(0,32));
      let c1 = leBytesToBigint(bytes.slice(32,64));
      let x = { c0: c0, c1: c1 };
      if (!bn254.G2.Point.Fp.isValid(x)) throw new Error('bad point: is not on curve, wrong x');
      let y_squared = weierstrassEquation(x, bn254.G2.Point);
      let y_or_neg_y;
      try {
        y_or_neg_y = bn254.G2.Point.Fp.sqrt(y_squared);
      } catch (sqrtError) {
        const err = sqrtError instanceof Error ? ': ' + sqrtError.message : '';
        throw new Error('bad point: is not on curve, sqrt error' + err);
      }
      let y = flag == SWFlag.YIsPositive ? 
        ensurePositiveFp2(y_or_neg_y, bn254.G2.Point) :
        ensureNegativeFp2(y_or_neg_y, bn254.G2.Point);
      return new bn254.G2.Point(x, y, bn254.G2.Point.Fp.ONE);
    }
  }
}
