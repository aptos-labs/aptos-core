// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
import { bn254 } from '@noble/curves/bn254.js';
import { bytesToG1, bytesToG2, g1ToBytes, g2ToBytes } from "../src/curveSerialization.js";
import { describe, expect, it } from 'vitest';


describe("Curve serialization", () => {

  it("g1ToBytes", () => {
    console.log(g1ToBytes(bn254.G1.Point.ZERO));
    let g1 = bn254.G1.Point.BASE;
    console.log(g1ToBytes(g1));
    g1 = g1.double();
    console.log(g1ToBytes(g1));
    g1 = g1.double();
    console.log(g1ToBytes(g1));
    g1 = g1.double();
    console.log(g1ToBytes(g1));
  });

  it("g2ToBytes", () => {
    console.log(g2ToBytes(bn254.G2.Point.ZERO));
    let g2 = bn254.G2.Point.BASE;
    console.log(g2ToBytes(g2));
    g2 = g2.double();
    console.log(g2ToBytes(g2));
    g2 = g2.double();
    console.log(g2ToBytes(g2));
    g2 = g2.double();
    console.log(g2ToBytes(g2));
  });

  it("g1ToBytesToG1", () => {
    let points = [bn254.G1.Point.ZERO, bn254.G1.Point.BASE];
    for (let i = 0; i < 10; i++) {
      points.push(points[points.length - 1].double());
    }

    points.forEach(function(p) {
      expect(bytesToG1(g1ToBytes(p)).toAffine())
        .toStrictEqual(p.toAffine());
      expect(bytesToG1(g1ToBytes(p.negate())).toAffine())
        .toStrictEqual(p.negate().toAffine());
    });

  });

  it("g2ToBytesToG2", () => {
    let points = [bn254.G2.Point.ZERO, bn254.G2.Point.BASE];
    for (let i = 0; i < 10; i++) {
      points.push(points[points.length - 1].double());
    }

    points.forEach(function(p) {
      expect(bytesToG2(g2ToBytes(p)).toAffine())
        .toStrictEqual(p.toAffine());
      expect(bytesToG2(g2ToBytes(p.negate())).toAffine())
        .toStrictEqual(p.negate().toAffine());
    });

  });
});
