const path = require("path");
const F1Field = require("ffjavascript").F1Field;
const Scalar = require("ffjavascript").Scalar;
exports.p = Scalar.fromString("21888242871839275222246405745257275088548364400416034343698204186575808495617");
const Fr = new F1Field(exports.p);
const wasm_tester = require("circom_tester").wasm;

function print(circuit, w, s) {
    console.log(s + ": " + w[circuit.getSignalIdx(s)]);
}

function getBits(v, n) {
    const res = [];
    for (let i=0; i<n; i++) {
        if (Scalar.isOdd(Scalar.shr(v, i))) {
            res.push(Fr.one);
        } else {
            res.push(Fr.zero);
        }
    }
    return res;
}

const q = Scalar.fromString("21888242871839275222246405745257275088548364400416034343698204186575808495617");

describe("Sign test", function() {
    let circuit;
    this.timeout(100000);

    before( async() => {
        circuit = await wasm_tester(path.join(__dirname, "circuits", "sign_test.circom"));
    });

    it("Sign of 0", async () => {
        const inp = getBits(Scalar.e(0), 254);
        const w = await circuit.calculateWitness({in: inp}, true);

        await circuit.assertOut(w, {sign: 0});
    });

    it("Sign of 3", async () => {
        const inp = getBits(Scalar.e(3), 254);
        const w = await circuit.calculateWitness({in: inp}, true);

        await circuit.assertOut(w, {sign: 0});
    });

    it("Sign of q/2", async () => {
        const inp = getBits(Scalar.shr(q, 1), 254);
        const w = await circuit.calculateWitness({in: inp}, true);

        await circuit.assertOut(w, {sign: 0});
    });

    it("Sign of q/2+1", async () => {
        const inp = getBits(Scalar.add(Scalar.shr(q, 1), 1) , 254);
        const w = await circuit.calculateWitness({in: inp}, true);

        await circuit.assertOut(w, {sign: 1});
    });

    it("Sign of q-1", async () => {
        const inp = getBits(Scalar.sub(q, 1), 254);
        const w = await circuit.calculateWitness({in: inp}, true);

        await circuit.assertOut(w, {sign: 1});
    });

    it("Sign of q", async () => {
        const inp = getBits(q, 254);
        const w = await circuit.calculateWitness({in: inp}, true);

        await circuit.assertOut(w, {sign: 1});
    });

    it("Sign of all ones", async () => {
        const inp = getBits(Scalar.sub(Scalar.shl(1,254),1), 254);
        const w = await circuit.calculateWitness({in: inp}, true);

        await circuit.assertOut(w, {sign: 1});
    });
});
