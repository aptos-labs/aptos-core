const path = require("path");
const wasm_tester = require("circom_tester").wasm;

const buildMimcSponge = require("circomlibjs").buildMimcSponge;


describe("MiMC Sponge Circuit test", function () {
    let circuit;
    let mimcSponge;
    let F;

    this.timeout(100000);

    before( async () => {
        mimcSponge = await buildMimcSponge();
        F = mimcSponge.F;
    });


    it("Should check permutation", async () => {

        circuit = await wasm_tester(path.join(__dirname, "circuits", "mimc_sponge_test.circom"));

        const w = await circuit.calculateWitness({xL_in: 1, xR_in: 2, k: 3});

        const out2 = mimcSponge.hash(1,2,3);

        await circuit.assertOut(w, {xL_out: F.toObject(out2.xL), xR_out: F.toObject(out2.xR)});

        await circuit.checkConstraints(w);

    });

    it("Should check hash", async () => {
        circuit = await wasm_tester(path.join(__dirname, "circuits", "mimc_sponge_hash_test.circom"));

        const w = await circuit.calculateWitness({ins: [1, 2], k: 0});

        const out2 = mimcSponge.multiHash([1,2], 0, 3);

        for (let i=0; i<out2.length; i++) out2[i] = F.toObject(out2[i]);

        await circuit.assertOut(w, {outs: out2});

        await circuit.checkConstraints(w);
    });
});
