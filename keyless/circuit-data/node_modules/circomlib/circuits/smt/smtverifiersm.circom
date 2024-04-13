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
Each level in the SMTVerifier has a state.

This is the state machine.

The signals are

levIns: 1 if we are in the level where the insertion should happen
xor: 1 if the bitKey of the old and new keys are different in this level
is0: Input that indicates that the oldKey is 0
fnc:  0 -> VERIFY INCLUSION
      1 -> VERIFY NOT INCLUSION

err state is not a state itself. It's a lack of state.

The end of the last level will have to be `na`

   levIns=0                                                                                any
     ┌────┐                                                                              ┌────┐
     │    │                                                                              │    │
     │    ▼                          levIns=1                                            ▼    │
     │    ###########                is0=1      ###########                    ###########    │
     │   #           #               fnc=1     #           #       any        #           #   │
     └──#    top      # ─────────────────────▶#     i0      #───────────────▶#    na       #──┘
         ##         ## ──────────┐             ##         ##         ┌───────▶##         ##
           ########─────────────┐│               #########           │┌────────▶#########
                                ││ levIns=1                          ││
                                ││ is0=0        ###########          ││
                                ││ fnc=1       #           #       any│
                                │└──────────▶ #    iold     #────────┘│
                                │              ##         ##          │
                                │                #########            │
                                │                                     │
                                │  levIns=1     ###########           │
                                │  fnc=0       #           #        any
                                └────────────▶#    inew     #─────────┘
                                               ##         ##
                                                 #########

 */
 pragma circom 2.0.0;


template SMTVerifierSM() {
    signal input is0;
    signal input levIns;
    signal input fnc;

    signal input prev_top;
    signal input prev_i0;
    signal input prev_iold;
    signal input prev_inew;
    signal input prev_na;

    signal output st_top;
    signal output st_i0;
    signal output st_iold;
    signal output st_inew;
    signal output st_na;

    signal prev_top_lev_ins;
    signal prev_top_lev_ins_fnc;

    prev_top_lev_ins <== prev_top * levIns;
    prev_top_lev_ins_fnc <== prev_top_lev_ins*fnc;  // prev_top * levIns * fnc

    // st_top = prev_top * (1-levIns)
    //    = + prev_top
    //      - prev_top * levIns
    st_top <== prev_top - prev_top_lev_ins;

    // st_inew = prev_top * levIns * (1-fnc)
    //   = + prev_top * levIns
    //     - prev_top * levIns * fnc
    st_inew <== prev_top_lev_ins - prev_top_lev_ins_fnc;

    // st_iold = prev_top * levIns * (1-is0)*fnc
    //   = + prev_top * levIns * fnc
    //     - prev_top * levIns * fnc * is0
    st_iold <== prev_top_lev_ins_fnc * (1 - is0);

    // st_i0 = prev_top * levIns * is0
    //  = + prev_top * levIns * is0
    st_i0 <== prev_top_lev_ins * is0;

    st_na <== prev_na + prev_inew + prev_iold + prev_i0;
}
