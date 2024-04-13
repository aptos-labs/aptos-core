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

include "../binsum.circom";
include "sigma.circom";
include "maj.circom";

template T2() {
    signal input a[32];
    signal input b[32];
    signal input c[32];
    signal output out[32];
    var k;

    component bigsigma0 = BigSigma(2, 13, 22);
    component maj = Maj_t(32);
    for (k=0; k<32; k++) {
        bigsigma0.in[k] <== a[k];
        maj.a[k] <== a[k];
        maj.b[k] <== b[k];
        maj.c[k] <== c[k];
    }

    component sum = BinSum(32, 2);

    for (k=0; k<32; k++) {
        sum.in[0][k] <== bigsigma0.out[k];
        sum.in[1][k] <== maj.out[k];
    }

    for (k=0; k<32; k++) {
        out[k] <== sum.out[k];
    }
}
