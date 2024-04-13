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

SMTProcessorLevel

This circuit has 2 hash

Outputs according to the state.

State        oldRoot                    newRoot
=====        =======                    =======
top          H'(oldChild, sibling)       H'(newChild, sibling)
old0         0                           new1leaf
bot          old1leaf                    H'(newChild, 0)
new1         old1leaf                    H'(new1leaf, old1leaf)
na           0                           0

upd          old1leaf                    new1leaf

H' is the Hash function with the inputs shifted acordingly.

*****/
pragma circom 2.0.0;


template SMTProcessorLevel() {
    signal input st_top;
    signal input st_old0;
    signal input st_bot;
    signal input st_new1;
    signal input st_na;
    signal input st_upd;

    signal output oldRoot;
    signal output newRoot;
    signal input sibling;
    signal input old1leaf;
    signal input new1leaf;
    signal input newlrbit;
    signal input oldChild;
    signal input newChild;

    signal aux[4];

    component oldProofHash = SMTHash2();
    component newProofHash = SMTHash2();

    component oldSwitcher = Switcher();
    component newSwitcher = Switcher();

    // Old side

    oldSwitcher.L <== oldChild;
    oldSwitcher.R <== sibling;

    oldSwitcher.sel <== newlrbit;
    oldProofHash.L <== oldSwitcher.outL;
    oldProofHash.R <== oldSwitcher.outR;

    aux[0] <== old1leaf * (st_bot + st_new1 + st_upd);
    oldRoot <== aux[0] +  oldProofHash.out * st_top;

    // New side

    aux[1] <== newChild * ( st_top + st_bot);
    newSwitcher.L <== aux[1] + new1leaf*st_new1;

    aux[2] <== sibling*st_top;
    newSwitcher.R <== aux[2] + old1leaf*st_new1;

    newSwitcher.sel <== newlrbit;
    newProofHash.L <== newSwitcher.outL;
    newProofHash.R <== newSwitcher.outR;

    aux[3] <== newProofHash.out * (st_top + st_bot + st_new1);
    newRoot <==  aux[3] + new1leaf * (st_old0 + st_upd);
}
