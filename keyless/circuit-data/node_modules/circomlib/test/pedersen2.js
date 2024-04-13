const path = require("path");

const Scalar = require("ffjavascript").Scalar;

const buildPedersenHash = require("circomlibjs").buildPedersenHash;
const buildBabyJub = require("circomlibjs").buildBabyjub;

const wasm_tester = require("circom_tester").wasm;


describe("Pedersen test", function() {
    let babyJub
    let pedersen;
    let F;
    let circuit;
    this.timeout(100000);
    before( async() => {

        babyJub = await buildBabyJub();
        F = babyJub.F;
        pedersen = await buildPedersenHash();
        circuit = await wasm_tester(path.join(__dirname, "circuits", "pedersen2_test.circom"));
    });
    it("Should pedersen at zero", async () => {

        let w;

        w = await circuit.calculateWitness({ in: 0}, true);

        const b = Buffer.alloc(32);

        const h = pedersen.hash(b);
        const hP = babyJub.unpackPoint(h);

        await circuit.assertOut(w, {out: [F.toObject(hP[0]), F.toObject(hP[1])] });

    });
    it("Should pedersen with 253 ones", async () => {

        let w;

        const n = F.e(Scalar.sub(Scalar.shl(Scalar.e(1), 253), Scalar.e(1)));

        w = await circuit.calculateWitness({ in: F.toObject(n)}, true);

        const b = Buffer.alloc(32);
        for (let i=0; i<31; i++) b[i] = 0xFF;
        b[31] = 0x1F;

        const h = pedersen.hash(b);
        const hP = babyJub.unpackPoint(h);

        await circuit.assertOut(w, {out: [F.toObject(hP[0]), F.toObject(hP[1])] });

    });
});
