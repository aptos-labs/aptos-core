/*
    Copyright 2018 0KIMS association.

    This file is part of circom (Zero Knowledge Circuit Compiler).

    circom is a free software: you can redistribute it and/or modify it
    under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    circom is distributed in the hope that it will be useful, but WITHOUT
    ANY WARRANTY; without even the implied warranty of MERCHANTABILITY
    or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public
    License for more details.

    You should have received a copy of the GNU General Public License
    along with circom. If not, see <https://www.gnu.org/licenses/>.
*/
pragma circom 2.0.0;

include "compconstant.circom";
include "pointbits.circom";
include "mimcsponge.circom";
include "bitify.circom";
include "escalarmulany.circom";
include "escalarmulfix.circom";

template EdDSAMiMCSpongeVerifier() {
    signal input enabled;
    signal input Ax;
    signal input Ay;

    signal input S;
    signal input R8x;
    signal input R8y;

    signal input M;

    var i;

// Ensure S<Subgroup Order

    component snum2bits = Num2Bits(253);
    snum2bits.in <== S;

    component  compConstant = CompConstant(2736030358979909402780800718157159386076813972158567259200215660948447373040);

    for (i=0; i<253; i++) {
        snum2bits.out[i] ==> compConstant.in[i];
    }
    compConstant.in[253] <== 0;
    compConstant.out === 0;

// Calculate the h = H(R,A, msg)

    component hash = MiMCSponge(5, 220, 1);
    hash.ins[0] <== R8x;
    hash.ins[1] <== R8y;
    hash.ins[2] <== Ax;
    hash.ins[3] <== Ay;
    hash.ins[4] <== M;
    hash.k <== 0;

    component h2bits = Num2Bits_strict();
    h2bits.in <== hash.outs[0];

// Calculate second part of the right side:  right2 = h*8*A

    // Multiply by 8 by adding it 3 times.  This also ensure that the result is in
    // the subgroup.
    component dbl1 = BabyDbl();
    dbl1.x <== Ax;
    dbl1.y <== Ay;
    component dbl2 = BabyDbl();
    dbl2.x <== dbl1.xout;
    dbl2.y <== dbl1.yout;
    component dbl3 = BabyDbl();
    dbl3.x <== dbl2.xout;
    dbl3.y <== dbl2.yout;

    // We check that A is not zero.
    component isZero = IsZero();
    isZero.in <== dbl3.x;
    isZero.out === 0;

    component mulAny = EscalarMulAny(254);
    for (i=0; i<254; i++) {
        mulAny.e[i] <== h2bits.out[i];
    }
    mulAny.p[0] <== dbl3.xout;
    mulAny.p[1] <== dbl3.yout;


// Compute the right side: right =  R8 + right2

    component addRight = BabyAdd();
    addRight.x1 <== R8x;
    addRight.y1 <== R8y;
    addRight.x2 <== mulAny.out[0];
    addRight.y2 <== mulAny.out[1];

// Calculate left side of equation left = S*B8

    var BASE8[2] = [
        5299619240641551281634865583518297030282874472190772894086521144482721001553,
        16950150798460657717958625567821834550301663161624707787222815936182638968203
    ];
    component mulFix = EscalarMulFix(253, BASE8);
    for (i=0; i<253; i++) {
        mulFix.e[i] <== snum2bits.out[i];
    }

// Do the comparation left == right if enabled;

    component eqCheckX = ForceEqualIfEnabled();
    eqCheckX.enabled <== enabled;
    eqCheckX.in[0] <== mulFix.out[0];
    eqCheckX.in[1] <== addRight.xout;

    component eqCheckY = ForceEqualIfEnabled();
    eqCheckY.enabled <== enabled;
    eqCheckY.in[0] <== mulFix.out[1];
    eqCheckY.in[1] <== addRight.yout;
}
