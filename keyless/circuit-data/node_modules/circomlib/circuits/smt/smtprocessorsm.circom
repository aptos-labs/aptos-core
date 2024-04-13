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

/***************************************************************************************************
Each level on a SMTProcessor has a state.

The state of the level depends on the state of te botom level and on `xor` and
`is0` signals.

`isOldLev` 1 when is the level where oldLeaf is.

`xor` signal is 0 if the index bit at the current level is the same in the old
and the new index, and 1 if it is different.

`is0` signal, is 1 if we are inserting/deleting in an empty leaf and 0 if we
are inserting/deleting in a leaf that contains an element.

The states are:

top: While the index bits of the old and new insex in the top level is the same, whe are in the top state.
old0: When the we reach insert level,  we go to old0 state
if `is0`=1.
btn: Once in insert level and `is0` =0 we go to btn or new1 level if xor=1
new1: This level is reached when xor=1. Here is where we insert/delete the hash of the
old and the new trees with just one element.
na: Not appliable.  After processing it, we go to the na level.


Fnction
fnc[0]  fnc[1]
0       0             NOP
0       1             UPDATE
1       0             INSERT
1       1             DELETE


                                                ###########
                                               #           #
                ┌────────────────────────────▶#    upd      #─────────────────────┐
                │                              ##         ##                      │
                │                                #########                        │
      levIns=1  │                                                                 │
      fnc[0]=0  │                                                                 │  any
                │                                                                 │
                │                                                                 │
                │                                                                 │
                │                                ###########                      │
                │        levIns=1               #           #                     │
   levIns=0     │         is0=1  ┌────────────▶#    old0     #────────┐           │     any
   ┌─────┐      │        fnc[0]=1│              ##         ##         │           │   ┌──────┐
   │     │      │                │                #########           │ any       │   │      │
   │     ▼      │                │                                    │           ▼   ▼      │
   │    ###########              │                                    │          ########### │
   │   #           # ────────────┘                                    └────────▶#           #│
   └──#    top      #                                                          #    na       #
       ##         ## ───────────────────┐ levIns=1                          ┌──▶##         ##
         #########                      │  is0=0                            │     #########
             │                          │ fnc[0]=1                          │
             │                          │  xor=1             ###########    │ any
             │                          └──────────────────▶#           #   │
             │                                             #    new1     #──┘
             │                                              ##         ##
             └────────────────────────────────┐               #########
                  levIns=1                    │                   ▲
                   is0=0                      │             ┌─────┘
                  fnc[0]=1                    │  ###########│  xor=1
                   xor=0                      │ #           #
                                              ▼#    btn      #
                                                ##         ##
                                                  #########◀───────┐
                                                      │            │
                                                      │            │
                                                      └────────────┘
                                                          xor=0

***************************************************************************************************/
pragma circom 2.0.0;

template SMTProcessorSM() {
  signal input xor;
  signal input is0;
  signal input levIns;
  signal input fnc[2];

  signal input prev_top;
  signal input prev_old0;
  signal input prev_bot;
  signal input prev_new1;
  signal input prev_na;
  signal input prev_upd;

  signal output st_top;
  signal output st_old0;
  signal output st_bot;
  signal output st_new1;
  signal output st_na;
  signal output st_upd;

  signal aux1;
  signal aux2;

  aux1 <==                  prev_top * levIns;
  aux2 <== aux1*fnc[0];  // prev_top * levIns * fnc[0]

  // st_top = prev_top*(1-levIns)
  //    = + prev_top
  //      - prev_top * levIns            = aux1

  st_top <== prev_top - aux1;

  // st_old0 = prev_top * levIns * is0 * fnc[0]
  //      = + prev_top * levIns * is0 * fnc[0]            = aux2 * is0

  st_old0 <== aux2 * is0;  // prev_top * levIns * is0 * fnc[0]

  // st_new1 = prev_top * levIns * (1-is0)*fnc[0] * xor   +  prev_bot*xor =
  //    = + prev_top * levIns *       fnc[0] * xor     = aux2     * xor
  //      - prev_top * levIns * is0 * fnc[0] * xor     = st_old0  * xor
  //      + prev_bot *                         xor     = prev_bot * xor

  st_new1 <== (aux2 - st_old0 + prev_bot)*xor;


  // st_bot = prev_top * levIns * (1-is0)*fnc[0] * (1-xor) + prev_bot*(1-xor);
  //    = + prev_top * levIns *       fnc[0]
  //      - prev_top * levIns * is0 * fnc[0]
  //      - prev_top * levIns *       fnc[0] * xor
  //      + prev_top * levIns * is0 * fnc[0] * xor
  //      + prev_bot
  //      - prev_bot *                         xor

  st_bot <== (1-xor) * (aux2 - st_old0 + prev_bot);


  // st_upd = prev_top * (1-fnc[0]) *levIns;
  //    = + prev_top * levIns
  //      - prev_top * levIns * fnc[0]

  st_upd <== aux1 - aux2;

  // st_na = prev_new1 + prev_old0 + prev_na + prev_upd;
  //    = + prev_new1
  //      + prev_old0
  //      + prev_na
  //      + prev_upd

  st_na <== prev_new1 + prev_old0 + prev_na + prev_upd;

}
