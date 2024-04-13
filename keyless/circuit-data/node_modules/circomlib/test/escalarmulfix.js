const chai = require("chai");
const path = require("path");
const wasm_tester = require("circom_tester").wasm;
const buildBabyjub = require("circomlibjs").buildBabyjub;
const Scalar = require("ffjavascript").Scalar;

const assert = chai.assert;

function print(circuit, w, s) {
    console.log(s + ": " + w[circuit.getSignalIdx(s)]);
}

describe("Escalarmul test", function () {
    let babyJub;
    let Fr;
    let circuit;

    this.timeout(100000);


    before( async() => {
        babyJub = await buildBabyjub();
        Fr = babyJub.F;
        circuit = await wasm_tester(path.join(__dirname, "circuits", "escalarmulfix_test.circom"));
    });

    it("Should generate Same escalar mul", async () => {

        const w = await circuit.calculateWitness({"e": 0});

        await circuit.checkConstraints(w);

        await circuit.assertOut(w, {out: [0,1]}, true);

    });

    it("Should generate Same escalar mul", async () => {

        const w = await circuit.calculateWitness({"e": 1}, true);

        await circuit.checkConstraints(w);

        await circuit.assertOut(w, {out: [Fr.toObject(babyJub.Base8[0]), Fr.toObject(babyJub.Base8[1])]});

    });

    it("Should generate scalar mul of a specific constant", async () => {

        const s = Scalar.e("2351960337287830298912035165133676222414898052661454064215017316447594616519");
        const base8 = [
            Fr.e("5299619240641551281634865583518297030282874472190772894086521144482721001553"),
            Fr.e("16950150798460657717958625567821834550301663161624707787222815936182638968203")
        ];

        const w = await circuit.calculateWitness({"e": s}, true);

        await circuit.checkConstraints(w);

        const expectedRes = babyJub.mulPointEscalar(base8, s);

        await circuit.assertOut(w, {out: [Fr.toObject(expectedRes[0]), Fr.toObject(expectedRes[1])]});

    });

    it("Should generate scalar mul of the firsts 50 elements", async () => {

        const base8 = [
            Fr.e("5299619240641551281634865583518297030282874472190772894086521144482721001553"),
            Fr.e("16950150798460657717958625567821834550301663161624707787222815936182638968203")
        ];

        for (let i=0; i<50; i++) {
            const s = Scalar.e(i);

            const w = await circuit.calculateWitness({"e": s}, true);

            await circuit.checkConstraints(w);

            const expectedRes = babyJub.mulPointEscalar(base8, s);

            await circuit.assertOut(w, {out: [Fr.toObject(expectedRes[0]), Fr.toObject(expectedRes[1])]});
        }
    });

    it("If multiply by order should return 0", async () => {

        const w = await circuit.calculateWitness({"e": babyJub.subOrder }, true);

        await circuit.checkConstraints(w);

        await circuit.assertOut(w, {out: [0,1]});
    });

});

