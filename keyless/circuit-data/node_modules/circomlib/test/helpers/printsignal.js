
const snarkjs = require("snarkjs");

const bigInt = snarkjs.bigInt;

module.exports = function hexBits(cir, witness, sig, nBits) {
    let v = bigInt(0);
    for (let i=nBits-1; i>=0; i--) {
        v = v.shiftLeft(1);
        const name = sig+"["+i+"]";
        const idx = cir.getSignalIdx(name);
        const vbit = bigInt(witness[idx].toString());
        if (vbit.equals(bigInt(1))) {
            v = v.add(bigInt(1));
        } else if (vbit.equals(bigInt(0))) {
            v;
        } else {
            console.log("Not Binary: "+name);
        }
    }
    return v.toString(16);
};
