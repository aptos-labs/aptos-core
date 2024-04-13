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

include "../mimc.circom";


/*
    Hash1 = H(1 | key | value)
 */

template SMTHash1() {
    signal input key;
    signal input value;
    signal output out;

    component h = MultiMiMC7(2, 91);   // Constant
    h.in[0] <== key;
    h.in[1] <== value;
    h.k <== 1;

    out <== h.out;
}

/*
    This component is used to create the 2 nodes.

    Hash2 = H(Hl | Hr)
 */

template SMTHash2() {
    signal input L;
    signal input R;
    signal output out;

    component h = MultiMiMC7(2, 91);   // Constant
    h.in[0] <== L;
    h.in[1] <== R;
    h.k <== 0;

    out <== h.out;
}
