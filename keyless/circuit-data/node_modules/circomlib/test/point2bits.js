const path = require("path");
const wasm_tester = require("circom_tester").wasm;
const buildBabyJub = require("circomlibjs").buildBabyjub;

const babyJub = require("circomlibjs").babyjub;


describe("Point 2 bits test", function() {
    let babyJub;
    let F;
    let circuit;
    this.timeout(100000);
    before( async() => {
        babyJub = await buildBabyJub();
        F = babyJub.F;

        circuit = await wasm_tester(path.join(__dirname, "circuits", "pointbits_loopback.circom"));
    });

    it("Should do the both convertions for 8Base", async () => {
        const w = await circuit.calculateWitness({ in: [F.toObject(babyJub.Base8[0]), F.toObject(babyJub.Base8[1])]}, true);

        await circuit.checkConstraints(w);
    });
    it("Should do the both convertions for Zero point", async () => {
        const w = await circuit.calculateWitness({ in: [0, 1]}, true);

        await circuit.checkConstraints(w);
    });
});
