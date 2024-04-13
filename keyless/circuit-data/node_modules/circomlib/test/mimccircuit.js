const chai = require("chai");
const path = require("path");
const wasm_tester = require("circom_tester").wasm;

const buildMimc7 = require("circomlibjs").buildMimc7;

describe("MiMC Circuit test", function () {
    let circuit;
    let mimc7;

    this.timeout(100000);

    before( async () => {
        mimc7 = await buildMimc7();
        circuit = await wasm_tester(path.join(__dirname, "circuits", "mimc_test.circom"));
    });

    it("Should check constrain", async () => {
        const w = await circuit.calculateWitness({x_in: 1, k: 2}, true);

        const res2 = mimc7.hash(1,2,91);

        await circuit.assertOut(w, {out: mimc7.F.toObject(res2)});

        await circuit.checkConstraints(w);
    });
});
