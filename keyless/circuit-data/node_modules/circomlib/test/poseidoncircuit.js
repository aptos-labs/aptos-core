const chai = require("chai");
const path = require("path");
const wasm_tester = require("circom_tester").wasm;

const buildPoseidon = require("circomlibjs").buildPoseidon;

const assert = chai.assert;

describe("Poseidon Circuit test", function () {
    let poseidon;
    let F;
    let circuit6;
    let circuit3;
    let circuitEx;

    this.timeout(1000000);

    before( async () => {
        poseidon = await buildPoseidon();
        F = poseidon.F;
        circuit6 = await wasm_tester(path.join(__dirname, "circuits", "poseidon6_test.circom"));
        circuit3 = await wasm_tester(path.join(__dirname, "circuits", "poseidon3_test.circom"));
        circuitEx = await wasm_tester(path.join(__dirname, "circuits", "poseidonex_test.circom"));
    });

    it("Should check constrain of hash([1, 2]) t=6", async () => {
        const w = await circuit6.calculateWitness({inputs: [1, 2, 0,0,0]}, true);

        const res2 = poseidon([1,2,0,0,0]);

        assert(F.eq(F.e("1018317224307729531995786483840663576608797660851238720571059489595066344487"), F.e(res2)));
        await circuit6.assertOut(w, {out : F.toObject(res2)});
        await circuit6.checkConstraints(w);
    });

    it("Should check constrain of hash([3, 4]) t=6", async () => {
        const w = await circuit6.calculateWitness({inputs: [3, 4,5,10,23]});

        const res2 = poseidon([3, 4,5,10,23]);

        assert(F.eq(F.e("13034429309846638789535561449942021891039729847501137143363028890275222221409"), F.e(res2)));
        await circuit6.assertOut(w, {out : F.toObject(res2)});
        await circuit6.checkConstraints(w);
    });


    it("Should check constrain of hash([1, 2]) t=3", async () => {
        const w = await circuit3.calculateWitness({inputs: [1, 2]});

        const res2 = poseidon([1,2]);

        assert(F.eq(F.e("7853200120776062878684798364095072458815029376092732009249414926327459813530"), F.e(res2)));
        await circuit3.assertOut(w, {out : F.toObject(res2)});
        await circuit3.checkConstraints(w);
    });

    it("Should check constrain of hash([3, 4]) t=3", async () => {
        const w = await circuit3.calculateWitness({inputs: [3, 4]});

        const res2 = poseidon([3, 4]);

        assert(F.eq(F.e("14763215145315200506921711489642608356394854266165572616578112107564877678998"), F.e(res2)));
        await circuit3.assertOut(w, {out : F.toObject(res2)});
        await circuit3.checkConstraints(w);
    });

    it("Should check constrain of hash with state and 16 ins and outs", async () => {
        const ins = [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16]
        const w = await circuitEx.calculateWitness({inputs: ins, initialState: 17});

        const res2 = poseidon(ins, 17, 17);
        const res2f = [];
        for (let i=0; i<res2.length; i++) {
            res2f[i] = F.toObject(res2[i]);
        }
        await circuitEx.assertOut(w, {out : res2f});
        await circuitEx.checkConstraints(w);
    });

});
