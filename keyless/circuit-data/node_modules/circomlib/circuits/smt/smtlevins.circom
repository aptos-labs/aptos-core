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

This component finds the level where the oldInsert is done.
The rules are:

levIns[i] == 1 if its level and all the child levels have a sibling of 0 and
the parent level has a sibling != 0.  Considere that the root level always has
a parent with a sibling != 0.


                                  ┌──────────────┐
                                  │              │
                                  │              │───▶ levIns[0] <== (1-done[i])
                                  │              │
                                  └──────────────┘
                                          ▲
                                          │
                                          │
                                       done[0]



                                      done[i-1] <== levIns[i] + done[i]
                                          ▲
                                          │
                                          │
                   ┌───────────┐  ┌──────────────┐
                   │           │  │              │
   sibling[i-1]───▶│IsZero[i-1]│─▶│              │───▶ levIns[i] <== (1-done[i])*(1-isZero[i-1].out)
                   │           │  │              │
                   └───────────┘  └──────────────┘
                                          ▲
                                          │
                                          │
                                       done[i]



                                     done[n-2] <== levIns[n-1]
                                         ▲
                                         │
                                         │
                  ┌───────────┐  ┌──────────────┐
                  │           │  │              │
  sibling[n-2]───▶│IsZero[n-2]│─▶│              │────▶ levIns[n-1] <== (1-isZero[n-2].out)
                  │           │  │              │
                  └───────────┘  └──────────────┘

                  ┌───────────┐
                  │           │
  sibling[n-1]───▶│IsZero[n-1]│────▶ === 0
                  │           │
                  └───────────┘

 */
 pragma circom 2.0.0;

template SMTLevIns(nLevels) {
    signal input enabled;
    signal input siblings[nLevels];
    signal output levIns[nLevels];
    signal done[nLevels-1];        // Indicates if the insLevel has aready been detected.

    var i;

    component isZero[nLevels];

    for (i=0; i<nLevels; i++) {
        isZero[i] = IsZero();
        isZero[i].in <== siblings[i];
    }

    // The last level must always have a sibling of 0. If not, then it cannot be inserted.
    (isZero[nLevels-1].out - 1) * enabled === 0;

    levIns[nLevels-1] <== (1-isZero[nLevels-2].out);
    done[nLevels-2] <== levIns[nLevels-1];
    for (i=nLevels-2; i>0; i--) {
        levIns[i] <== (1-done[i])*(1-isZero[i-1].out);
        done[i-1] <== levIns[i] + done[i];
    }

    levIns[0] <== (1-done[0]);
}
