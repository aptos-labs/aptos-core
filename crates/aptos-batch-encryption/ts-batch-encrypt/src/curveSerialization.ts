// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
import type { Fp2 } from '@noble/curves/abstract/tower.js';
import type { WeierstrassPoint, WeierstrassPointCons } from '@noble/curves/abstract/weierstrass.js';
import { bls12_381 } from '@noble/curves/bls12-381.js';
import { leBytesToBigint, bigintToLEBytesFq, bigintToLEBytesFr } from './fieldSerialization.ts';
import { bls } from '@noble/curves/abstract/bls.js';


export const G1_SIZE = 48;
export const G2_SIZE = 96;


export function weierstrassEquation<T>(x: T, p: WeierstrassPointCons<T>): T {
  const x2 = p.Fp.sqr(x); // x * x
  const x3 = p.Fp.mul(x2, x); // x² * x
  return p.Fp.add(p.Fp.add(x3, p.Fp.mul(x, p.CURVE().a)), p.CURVE().b); // x³ + a * x + b
}


export function g1ToBytes(p: WeierstrassPoint<bigint>): Uint8Array {
  return p.toBytes(true);
}


export function bytesToG1(bytes: Uint8Array): WeierstrassPoint<bigint> {
  return bls12_381.G1.Point.fromBytes(bytes);
}


export function g2ToBytes(p: WeierstrassPoint<Fp2>): Uint8Array {
  return p.toBytes(true);
}

export function bytesToG2(bytes: Uint8Array): WeierstrassPoint<Fp2> {
  return bls12_381.G2.Point.fromBytes(bytes);
}
