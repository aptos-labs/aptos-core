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

template SigmaPlus() {
    signal input in2[32];
    signal input in7[32];
    signal input in15[32];
    signal input in16[32];
    signal output out[32];
    var k;

    component sigma1 = SmallSigma(17,19,10);
    component sigma0 = SmallSigma(7, 18, 3);
    for (k=0; k<32; k++) {
        sigma1.in[k] <== in2[k];
        sigma0.in[k] <== in15[k];
    }

    component sum = BinSum(32, 4);
    for (k=0; k<32; k++) {
        sum.in[0][k] <== sigma1.out[k];
        sum.in[1][k] <== in7[k];
        sum.in[2][k] <== sigma0.out[k];
        sum.in[3][k] <== in16[k];
    }

    for (k=0; k<32; k++) {
        out[k] <== sum.out[k];
    }
}
