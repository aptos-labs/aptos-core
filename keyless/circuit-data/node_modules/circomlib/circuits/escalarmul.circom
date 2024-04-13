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

                                                        ┏━━━━━━━━━━━┓
                                                        ┃           ┃
                                                        ┃           ┃
  (inx, iny) ══════════════════════════════════════════▶┃ EC Point  ┃
                                                        ┃           ╠═▶ (outx, outy)
                                                    ╔══▶┃   Adder   ┃
                                                    ║   ┃           ┃
                                                    ║   ┃           ┃
                                                    ║   ┃           ┃
       ┏━━━━━━━━━━━┓                ┏━━━━━━━━━━━━┓  ║   ┗━━━━━━━━━━━┛
       ┃           ┃                ┃            ┃  ║
       ┃           ┃                ┃            ┃  ║
       ┃           ╠═══(p0x,p0y)═══▶┃            ┃  ║
       ┃           ╠═══(p1x,p1y)═══▶┃            ┃  ║
       ┃           ╠═══(p2x,p2y)═══▶┃            ┃  ║
       ┃           ╠═══(p3x,p3y)═══▶┃            ┃  ║
       ┃           ╠═══(p4x,p4y)═══▶┃            ┃  ║
       ┃           ╠═══(p5x,p5y)═══▶┃            ┃  ║
       ┃           ╠═══(p6x,p6y)═══▶┃            ┃  ║
       ┃ Constant  ╠═══(p7x,p7y)═══▶┃            ┃  ║
       ┃  Points   ┃                ┃    Mux4    ╠══╝
       ┃           ╠═══(p8x,p8y)═══▶┃            ┃
       ┃           ╠═══(p9x,p9y)═══▶┃            ┃
       ┃           ╠══(p10x,p10y)══▶┃            ┃
       ┃           ╠══(p11x,p11y)══▶┃            ┃
       ┃           ╠══(p12x,p12y)══▶┃            ┃
       ┃           ╠══(p13x,p13y)══▶┃            ┃
       ┃           ╠══(p14x,p14y)══▶┃            ┃
       ┃           ╠══(p15x,p15y)══▶┃            ┃
       ┃           ┃                ┃            ┃
       ┃           ┃                ┃            ┃
       ┗━━━━━━━━━━━┛                ┗━━━━━━━━━━━━┛
                                      ▲  ▲  ▲  ▲
                                      │  │  │  │
  s0 ─────────────────────────────────┘  │  │  │
  s1 ────────────────────────────────────┘  │  │
  s2 ───────────────────────────────────────┘  │
  s3 ──────────────────────────────────────────┘


 */
pragma circom 2.0.0;

include "mux4.circom";
include "escalarmulw4table.circom";
include "babyjub.circom";

template EscalarMulWindow(base, k) {

    signal input in[2];
    signal input sel[4];
    signal output out[2];

    var table[16][2];
    component mux;
    component adder;

    var i;

    table = EscalarMulW4Table(base, k);
    mux = MultiMux4(2);
    adder = BabyAdd();

    for (i=0; i<4; i++) {
        sel[i] ==> mux.s[i];
    }

    for (i=0; i<16; i++) {
        mux.c[0][i] <== table[i][0];
        mux.c[1][i] <== table[i][1];
    }

    in[0] ==> adder.x1;
    in[1] ==> adder.y1;

    mux.out[0] ==> adder.x2;
    mux.out[1] ==> adder.y2;

    adder.xout ==> out[0];
    adder.yout ==> out[1];
}

/*


                ┏━━━━━━━━━┓      ┏━━━━━━━━━┓                            ┏━━━━━━━━━━━━━━━━━━━┓
                ┃         ┃      ┃         ┃                            ┃                   ┃
      inp  ════▶┃Window(0)┃═════▶┃Window(1)┃════════  . . . . ═════════▶┃ Window(nBlocks-1) ┃═════▶ out
                ┃         ┃      ┃         ┃                            ┃                   ┃
                ┗━━━━━━━━━┛      ┗━━━━━━━━━┛                            ┗━━━━━━━━━━━━━━━━━━━┛
                  ▲ ▲ ▲ ▲          ▲ ▲ ▲ ▲                                    ▲ ▲ ▲ ▲
    in[0]─────────┘ │ │ │          │ │ │ │                                    │ │ │ │
    in[1]───────────┘ │ │          │ │ │ │                                    │ │ │ │
    in[2]─────────────┘ │          │ │ │ │                                    │ │ 0 0
    in[3]───────────────┘          │ │ │ │                                    │ │
    in[4]──────────────────────────┘ │ │ │                                    │ │
    in[5]────────────────────────────┘ │ │                                    │ │
    in[6]──────────────────────────────┘ │                                    │ │
    in[7]────────────────────────────────┘                                    │ │
        .                                                                     │ │
        .                                                                     │ │
  in[n-2]─────────────────────────────────────────────────────────────────────┘ │
  in[n-1]───────────────────────────────────────────────────────────────────────┘

 */

template EscalarMul(n, base) {
    signal input in[n];
    signal input inp[2];   // Point input to be added
    signal output out[2];

    var nBlocks = ((n-1)>>2)+1;
    var i;
    var j;

    component windows[nBlocks];

    // Construct the windows
    for (i=0; i<nBlocks; i++) {
      windows[i] = EscalarMulWindow(base, i);
    }

    // Connect the selectors
    for (i=0; i<nBlocks; i++) {
        for (j=0; j<4; j++) {
            if (i*4+j >= n) {
                windows[i].sel[j] <== 0;
            } else {
                windows[i].sel[j] <== in[i*4+j];
            }
        }
    }

    // Start with generator
    windows[0].in[0] <== inp[0];
    windows[0].in[1] <== inp[1];

    for(i=0; i<nBlocks-1; i++) {
        windows[i].out[0] ==> windows[i+1].in[0];
        windows[i].out[1] ==> windows[i+1].in[1];
    }

    windows[nBlocks-1].out[0] ==> out[0];
    windows[nBlocks-1].out[1] ==> out[1];
}
