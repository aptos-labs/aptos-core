const chai = require("chai");
const path = require("path");
const wasm_tester = require("circom_tester").wasm;
const buildBabyjub = require("circomlibjs").buildBabyjub;

const Scalar = require("ffjavascript").Scalar;

const assert = chai.assert;

function print(circuit, w, s) {
    console.log(s + ": " + w[circuit.getSignalIdx(s)]);
}

describe("Exponentioation test", function () {
    let babyJub;
    let Fr;
    this.timeout(100000);

    before( async () => {
        babyJub = await buildBabyjub();
        Fr = babyJub.F;
    });

    it("Should generate the Exponentiation table in k=0", async () => {

        const circuit = await wasm_tester(path.join(__dirname, "circuits", "escalarmulw4table_test.circom"));

        const w = await circuit.calculateWitness({in: 1});

        await circuit.checkConstraints(w);

        let g = [
            Fr.e("5299619240641551281634865583518297030282874472190772894086521144482721001553"),
            Fr.e("16950150798460657717958625567821834550301663161624707787222815936182638968203")
        ];

        let dbl= [Fr.e("0"), Fr.e("1")];

        const expectedOut = [];

        for (let i=0; i<16; i++) {

            expectedOut.push([Fr.toObject(dbl[0]), Fr.toObject(dbl[1])]);
            dbl = babyJub.addPoint(dbl,g);
        }

        await circuit.assertOut(w, {out: expectedOut});

    });

    it("Should generate the Exponentiation table in k=3", async () => {

        const circuit = await wasm_tester(path.join(__dirname, "circuits", "escalarmulw4table_test3.circom"));

        const w = await circuit.calculateWitness({in: 1});

        await circuit.checkConstraints(w);

        let g = [
            Fr.e("5299619240641551281634865583518297030282874472190772894086521144482721001553"),
            Fr.e("16950150798460657717958625567821834550301663161624707787222815936182638968203")
        ];

        for (let i=0; i<12;i++) {
            g = babyJub.addPoint(g,g);
        }

        let dbl= [Fr.e("0"), Fr.e("1")];

        const expectedOut = [];

        for (let i=0; i<16; i++) {
            expectedOut.push([Fr.toObject(dbl[0]), Fr.toObject(dbl[1])]);

            dbl = babyJub.addPoint(dbl,g);
        }

        await circuit.assertOut(w, {out: expectedOut});

    });

    it("Should exponentiate g^31", async () => {

        const circuit = await wasm_tester(path.join(__dirname, "circuits", "escalarmul_test.circom"));

        const w = await circuit.calculateWitness({"in": 31});

        await circuit.checkConstraints(w);

        let g = [
            Fr.e("5299619240641551281634865583518297030282874472190772894086521144482721001553"),
            Fr.e("16950150798460657717958625567821834550301663161624707787222815936182638968203")
        ];

        let c = [Fr.e(0), Fr.e(1)];

        for (let i=0; i<31;i++) {
            c = babyJub.addPoint(c,g);
        }

        await circuit.assertOut(w, {out: [Fr.toObject(c[0]), Fr.toObject(c[1])] });

        const w2 = await circuit.calculateWitness({"in": Scalar.add(Scalar.shl(Scalar.e(1), 252),Scalar.e(1))});

        c = [g[0], g[1]];
        for (let i=0; i<252;i++) {
            c = babyJub.addPoint(c,c);
        }
        c = babyJub.addPoint(c,g);

        await circuit.assertOut(w2, {out: [Fr.toObject(c[0]), Fr.toObject(c[1])] });

    }).timeout(10000000);

    it("Number of constrains for 256 bits", async () => {

        const circuit = await wasm_tester(path.join(__dirname, "circuits", "escalarmul_test_min.circom"));

    }).timeout(10000000);

});
