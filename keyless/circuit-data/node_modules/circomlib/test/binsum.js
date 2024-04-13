const chai = require("chai");
const path = require("path");

const wasm_tester = require("circom_tester").wasm;

const F1Field = require("ffjavascript").F1Field;
const Scalar = require("ffjavascript").Scalar;
exports.p = Scalar.fromString("21888242871839275222246405745257275088548364400416034343698204186575808495617");
const Fr = new F1Field(exports.p);

const assert = chai.assert;

describe("Binary sum test", function () {
    this.timeout(100000000);

    it("Should create a constant circuit", async () => {
        const circuit = await wasm_tester(path.join(__dirname, "circuits", "constants_test.circom"));
        await circuit.loadConstraints();
        assert.equal(circuit.nVars, 2);
        assert.equal(circuit.constraints.length, 1);

        const witness = await circuit.calculateWitness({ "in": Fr.toString(Fr.e("0xd807aa98"))}, true);

        assert(Fr.eq(Fr.e(witness[0]),Fr.e(1)));
        assert(Fr.eq(Fr.e(witness[1]),Fr.e("0xd807aa98")));
    });
    it("Should create a sum circuit", async () => {
        const circuit = await wasm_tester(path.join(__dirname, "circuits", "sum_test.circom"));
        await circuit.loadConstraints();

        assert.equal(circuit.constraints.length, 97);  // 32 (in1) + 32(in2) + 32(out) + 1 (carry)

        const witness = await circuit.calculateWitness({ "a": "111", "b": "222" }, true);

        assert(Fr.eq(Fr.e(witness[0]),Fr.e(1)));
        assert(Fr.eq(Fr.e(witness[1]),Fr.e("333")));
    });
});
