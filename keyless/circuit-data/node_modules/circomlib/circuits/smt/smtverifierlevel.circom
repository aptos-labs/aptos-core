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

/******

SMTVerifierLevel

This circuit has 1 hash

Outputs according to the state.

State        root
=====        =======
top          H'(child, sibling)
i0           0
iold         old1leaf
inew         new1leaf
na           0

H' is the Hash function with the inputs shifted acordingly.

*****/
pragma circom 2.0.0;

template SMTVerifierLevel() {
    signal input st_top;
    signal input st_i0;
    signal input st_iold;
    signal input st_inew;
    signal input st_na;

    signal output root;
    signal input sibling;
    signal input old1leaf;
    signal input new1leaf;
    signal input lrbit;
    signal input child;

    signal aux[2];

    component proofHash = SMTHash2();
    component switcher = Switcher();

    switcher.L <== child;
    switcher.R <== sibling;

    switcher.sel <== lrbit;
    proofHash.L <== switcher.outL;
    proofHash.R <== switcher.outR;

    aux[0] <== proofHash.out * st_top;
    aux[1] <== old1leaf*st_iold;

    root <== aux[0] + aux[1] + new1leaf*st_inew;
}
