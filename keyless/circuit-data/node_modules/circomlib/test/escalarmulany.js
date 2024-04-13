const chai = require("chai");
const path = require("path");
const wasm_tester = require("circom_tester").wasm;

const F1Field = require("ffjavascript").F1Field;
const Scalar = require("ffjavascript").Scalar;
exports.p = Scalar.fromString("21888242871839275222246405745257275088548364400416034343698204186575808495617");
const Fr = new F1Field(exports.p);

function print(circuit, w, s) {
    console.log(s + ": " + w[circuit.getSignalIdx(s)]);
}

describe("Escalarmul test", function () {
    let circuitEMulAny;

    this.timeout(100000);

    let g;

    before( async() => {
        circuitEMulAny = await wasm_tester(path.join(__dirname, "circuits", "escalarmulany_test.circom"));
        g = [
                Fr.e("5299619240641551281634865583518297030282874472190772894086521144482721001553"),
                Fr.e("16950150798460657717958625567821834550301663161624707787222815936182638968203")
            ]
    });

    it("Should generate Same escalar mul", async () => {

        const w = await circuitEMulAny.calculateWitness({"e": 1, "p": g});

        await circuitEMulAny.checkConstraints(w);

        await circuitEMulAny.assertOut(w, {out: g}, true);

    });

    it("If multiply by order should return 0", async () => {

        const r = Fr.e("2736030358979909402780800718157159386076813972158567259200215660948447373041");
        const w = await circuitEMulAny.calculateWitness({"e": r, "p": g});

        await circuitEMulAny.checkConstraints(w);

        await circuitEMulAny.assertOut(w, {out: [0,1]}, true);

    });

});

