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

/*
    Source: https://en.wikipedia.org/wiki/Montgomery_curve

                1 + y       1 + y
    [u, v] = [ -------  , ---------- ]
                1 - y      (1 - y)x

 */
 pragma circom 2.0.0;

template Edwards2Montgomery() {
    signal input in[2];
    signal output out[2];

    out[0] <-- (1 + in[1]) / (1 - in[1]);
    out[1] <-- out[0] / in[0];


    out[0] * (1-in[1]) === (1 + in[1]);
    out[1] * in[0] === out[0];
}

/*

                u    u - 1
    [x, y] = [ ---, ------- ]
                v    u + 1

 */
template Montgomery2Edwards() {
    signal input in[2];
    signal output out[2];

    out[0] <-- in[0] / in[1];
    out[1] <-- (in[0] - 1) / (in[0] + 1);

    out[0] * in[1] === in[0];
    out[1] * (in[0] + 1) === in[0] - 1;
}


/*
             x2 - x1
    lamda = ---------
             y2 - y1

                                                    x3 + A + x1 + x2
    x3 = B * lamda^2 - A - x1 -x2    =>  lamda^2 = ------------------
                                                         B

    y3 = (2*x1 + x2 + A)*lamda - B*lamda^3 - y1  =>


    =>  y3 = lamda * ( 2*x1 + x2 + A  - x3 - A - x1 - x2)  - y1 =>

    =>  y3 = lamda * ( x1 - x3 ) - y1

----------

             y2 - y1
    lamda = ---------
             x2 - x1

    x3 = B * lamda^2 - A - x1 -x2

    y3 = lamda * ( x1 - x3 ) - y1

 */

template MontgomeryAdd() {
    signal input in1[2];
    signal input in2[2];
    signal output out[2];

    var a = 168700;
    var d = 168696;

    var A = (2 * (a + d)) / (a - d);
    var B = 4 / (a - d);

    signal lamda;

    lamda <-- (in2[1] - in1[1]) / (in2[0] - in1[0]);
    lamda * (in2[0] - in1[0]) === (in2[1] - in1[1]);

    out[0] <== B*lamda*lamda - A - in1[0] -in2[0];
    out[1] <== lamda * (in1[0] - out[0]) - in1[1];
}

/*

    x1_2 = x1*x1

             3*x1_2 + 2*A*x1 + 1
    lamda = ---------------------
                   2*B*y1

    x3 = B * lamda^2 - A - x1 -x1

    y3 = lamda * ( x1 - x3 ) - y1

 */
template MontgomeryDouble() {
    signal input in[2];
    signal output out[2];

    var a = 168700;
    var d = 168696;

    var A = (2 * (a + d)) / (a - d);
    var B = 4 / (a - d);

    signal lamda;
    signal x1_2;

    x1_2 <== in[0] * in[0];

    lamda <-- (3*x1_2 + 2*A*in[0] + 1 ) / (2*B*in[1]);
    lamda * (2*B*in[1]) === (3*x1_2 + 2*A*in[0] + 1 );

    out[0] <== B*lamda*lamda - A - 2*in[0];
    out[1] <== lamda * (in[0] - out[0]) - in[1];
}
