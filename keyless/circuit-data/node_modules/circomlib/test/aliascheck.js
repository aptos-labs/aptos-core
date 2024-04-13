const chai = require("chai");
const path = require("path");

const assert = chai.assert;

const Scalar = require("ffjavascript").Scalar;
const F1Field = require("ffjavascript").F1Field;
const utils = require("ffjavascript").utils;
const q = Scalar.fromString("21888242871839275222246405745257275088548364400416034343698204186575808495617");
const F = new F1Field(q);

const wasm_tester = require("circom_tester").wasm;

function print(circuit, w, s) {
    console.log(s + ": " + w[circuit.getSignalIdx(s)]);
}

function getBits(v, n) {
    const res = [];
    for (let i=0; i<n; i++) {
        if (Scalar.isOdd(Scalar.shr(v,i))) {
            res.push(F.one);
        } else {
            res.push(F.zero);
        }
    }
    return res;
}


describe("Aliascheck test", function () {
    this.timeout(100000);

    let cir;
    before( async() => {

        cir = await wasm_tester(path.join(__dirname, "circuits", "aliascheck_test.circom"));
    });

    it("Satisfy the aliastest 0", async () => {
        const inp = getBits(0, 254);
        await cir.calculateWitness({in: inp}, true);
    });

    it("Satisfy the aliastest 3", async () => {
        const inp = getBits(3, 254);
        await cir.calculateWitness({in: inp}, true);
    });

    it("Satisfy the aliastest q-1", async () => {
        const inp = getBits(F.e(-1), 254);
        // console.log(JSON.stringify(utils.stringifyBigInts(inp)));
        await cir.calculateWitness({in: inp}, true);
    });

    it("Should not satisfy an input of q", async () => {
        const inp = getBits(q, 254);
        try {
            await cir.calculateWitness({in: inp}, true);
            assert(false);
        } catch(err) {
            assert(err.message.includes("Assert Failed"));
        }
    });

    it("Should not satisfy all ones", async () => {

        const inp = getBits(Scalar.sub(Scalar.shl(1, 254) , 1) , 254);
        try {
            await cir.calculateWitness({in: inp}, true);
            assert(false);
        } catch(err) {
            assert(err.message.includes("Assert Failed"));
        }
    });

});
