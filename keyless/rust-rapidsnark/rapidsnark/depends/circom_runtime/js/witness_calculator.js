/* globals WebAssembly */
/*

Copyright 2020 0KIMS association.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

*/

import { flatArray, fnvHash } from "./utils.js";
import { Scalar, F1Field } from "ffjavascript";

export default async function builder(code, options) {

    options = options || {};

    let memorySize = 32767;
    let memory;
    let memoryAllocated = false;
    while (!memoryAllocated){
        try{
            memory = new WebAssembly.Memory({initial:memorySize});
            memoryAllocated = true;
        } catch(err){
            if(memorySize === 1){
                throw err;
            }
            console.warn("Could not allocate " + memorySize * 1024 * 64 + " bytes. This may cause severe instability. Trying with " + memorySize * 1024 * 64 / 2 + " bytes");
            memorySize = Math.floor(memorySize/2);
        }
    }

    const wasmModule = await WebAssembly.compile(code);

    let wc;

    const instance = await WebAssembly.instantiate(wasmModule, {
        env: {
            "memory": memory
        },
        runtime: {
            error: function(code, pstr, a,b,c,d) {
                let errStr;
                if (code == 7) {
                    errStr=p2str(pstr) + " " + wc.getFr(b).toString() + " != " + wc.getFr(c).toString() + " " +p2str(d);
                } else if (code == 9) {
                    errStr=p2str(pstr) + " " + wc.getFr(b).toString() + " " +p2str(c);
                } else if ((code == 5)&&(options.sym)) {
                    errStr=p2str(pstr)+ " " + options.sym.labelIdx2Name[c];
                } else {
                    errStr=p2str(pstr)+ " " + a + " " + b + " " + c + " " + d;
                }
                console.log("ERROR: ", code, errStr);
                throw new Error(errStr);
            },
            log: function(a) {
                console.log(wc.getFr(a).toString());
            },
            logGetSignal: function(signal, pVal) {
                if (options.logGetSignal) {
                    options.logGetSignal(signal, wc.getFr(pVal) );
                }
            },
            logSetSignal: function(signal, pVal) {
                if (options.logSetSignal) {
                    options.logSetSignal(signal, wc.getFr(pVal) );
                }
            },
            logStartComponent: function(cIdx) {
                if (options.logStartComponent) {
                    options.logStartComponent(cIdx);
                }
            },
            logFinishComponent: function(cIdx) {
                if (options.logFinishComponent) {
                    options.logFinishComponent(cIdx);
                }
            }
        }
    });

    const sanityCheck =
        options &&
        (
            options.sanityCheck ||
            options.logGetSignal ||
            options.logSetSignal ||
            options.logStartComponent ||
            options.logFinishComponent
        );

    wc = new WitnessCalculator(memory, instance, sanityCheck);
    return wc;

    function p2str(p) {
        const i8 = new Uint8Array(memory.buffer);

        const bytes = [];

        for (let i=0; i8[p+i]>0; i++)  bytes.push(i8[p+i]);

        return String.fromCharCode.apply(null, bytes);
    }
};

class WitnessCalculator {
    constructor(memory, instance, sanityCheck) {
        this.memory = memory;
        this.i32 = new Uint32Array(memory.buffer);
        this.instance = instance;

        this.n32 = (this.instance.exports.getFrLen() >> 2) - 2;
        const pRawPrime = this.instance.exports.getPRawPrime();

        const arr = new Array(this.n32);
        for (let i=0; i<this.n32; i++) {
            arr[this.n32-1-i] = this.i32[(pRawPrime >> 2) + i];
        }

        this.prime = Scalar.fromArray(arr, 0x100000000);

        this.Fr = new F1Field(this.prime);

        this.mask32 = Scalar.fromString("FFFFFFFF", 16);
        this.NVars = this.instance.exports.getNVars();
        this.n64 = Math.floor((this.Fr.bitLength - 1) / 64)+1;
        this.R = this.Fr.e( Scalar.shiftLeft(1 , this.n64*64));
        this.RInv = this.Fr.inv(this.R);
        this.sanityCheck = sanityCheck;
    }

    async _doCalculateWitness(input, sanityCheck) {
        this.instance.exports.init((this.sanityCheck || sanityCheck) ? 1 : 0);
        const pSigOffset = this.allocInt();
        const pFr = this.allocFr();
        const keys = Object.keys(input);
        keys.forEach( (k) => {
            const h = fnvHash(k);
            const hMSB = parseInt(h.slice(0,8), 16);
            const hLSB = parseInt(h.slice(8,16), 16);
            try {
                this.instance.exports.getSignalOffset32(pSigOffset, 0, hMSB, hLSB);
            } catch (err) {
                throw new Error(`Signal ${k} is not an input of the circuit.`);
            }
            const sigOffset = this.getInt(pSigOffset);
            const fArr = flatArray(input[k]);
            for (let i=0; i<fArr.length; i++) {
                this.setFr(pFr, fArr[i]);
                this.instance.exports.setSignal(0, 0, sigOffset + i, pFr);
            }
        });
    }

    async calculateWitness(input, sanityCheck) {
        const self = this;

        const old0 = self.i32[0];
        const w = [];

        await self._doCalculateWitness(input, sanityCheck);

        for (let i=0; i<self.NVars; i++) {
            const pWitness = self.instance.exports.getPWitness(i);
            w.push(self.getFr(pWitness));
        }

        self.i32[0] = old0;
        return w;
    }

    async calculateBinWitness(input, sanityCheck) {
        const self = this;

        const old0 = self.i32[0];

        await self._doCalculateWitness(input, sanityCheck);

        const pWitnessBuffer = self.instance.exports.getWitnessBuffer();

        self.i32[0] = old0;

        const buff = self.memory.buffer.slice(pWitnessBuffer, pWitnessBuffer + (self.NVars * self.n64 * 8));
        return new Uint8Array(buff);
    }

    allocInt() {
        const p = this.i32[0];
        this.i32[0] = p+8;
        return p;
    }

    allocFr() {
        const p = this.i32[0];
        this.i32[0] = p+this.n32*4 + 8;
        return p;
    }

    getInt(p) {
        return this.i32[p>>2];
    }

    setInt(p, v) {
        this.i32[p>>2] = v;
    }

    getFr(p) {
        const self = this;
        const idx = (p>>2);

        if (self.i32[idx + 1] & 0x80000000) {
            const arr = new Array(self.n32);
            for (let i=0; i<self.n32; i++) {
                arr[self.n32-1-i] = self.i32[idx+2+i];
            }
            const res = self.Fr.e(Scalar.fromArray(arr, 0x100000000));
            if (self.i32[idx + 1] & 0x40000000) {
                return fromMontgomery(res);
            } else {
                return res;
            }

        } else {
            if (self.i32[idx] & 0x80000000) {
                return self.Fr.e( self.i32[idx] - 0x100000000);
            } else {
                return self.Fr.e(self.i32[idx]);
            }
        }

        function fromMontgomery(n) {
            return self.Fr.mul(self.RInv, n);
        }

    }


    setFr(p, v) {
        const self = this;

        v = self.Fr.e(v);

        const minShort = self.Fr.neg(self.Fr.e("80000000", 16));
        const maxShort = self.Fr.e("7FFFFFFF", 16);

        if (  (self.Fr.geq(v, minShort))
            &&(self.Fr.leq(v, maxShort)))
        {
            let a;
            if (self.Fr.geq(v, self.Fr.zero)) {
                a = Scalar.toNumber(v);
            } else {
                a = Scalar.toNumber( self.Fr.sub(v, minShort));
                a = a - 0x80000000;
                a = 0x100000000 + a;
            }
            self.i32[(p >> 2)] = a;
            self.i32[(p >> 2) + 1] = 0;
            return;
        }

        self.i32[(p >> 2)] = 0;
        self.i32[(p >> 2) + 1] = 0x80000000;
        const arr = Scalar.toArray(v, 0x100000000);
        for (let i=0; i<self.n32; i++) {
            const idx = arr.length-1-i;

            if ( idx >=0) {
                self.i32[(p >> 2) + 2 + i] = arr[idx];
            } else {
                self.i32[(p >> 2) + 2 + i] = 0;
            }
        }
    }
}



