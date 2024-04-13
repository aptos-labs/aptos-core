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

/* Ch

000 0
001 1
010 0
011 1
100 0
101 0
110 1
111 1

out = a&b ^ (!a)&c =>

out = a*(b-c) + c

*/
pragma circom 2.0.0;

template Ch_t(n) {
    signal input a[n];
    signal input b[n];
    signal input c[n];
    signal output out[n];

    for (var k=0; k<n; k++) {
        out[k] <== a[k] * (b[k]-c[k]) + c[k];
    }
}
