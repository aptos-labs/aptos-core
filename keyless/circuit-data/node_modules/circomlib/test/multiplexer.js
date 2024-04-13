const path = require("path");
const wasm_tester = require("circom_tester").wasm;
const F1Field = require("ffjavascript").F1Field;
const Scalar = require("ffjavascript").Scalar;
exports.p = Scalar.fromString("21888242871839275222246405745257275088548364400416034343698204186575808495617");
const Fr = new F1Field(exports.p);

describe("Mux4 test", function() {
    this.timeout(100000);
    it("Should create a constant multiplexer 4", async () => {

        const circuit = await wasm_tester(path.join(__dirname, "circuits", "mux4_1.circom"));

        const ct16 = [
            Fr.e("123"),
            Fr.e("456"),
            Fr.e("789"),
            Fr.e("012"),
            Fr.e("111"),
            Fr.e("222"),
            Fr.e("333"),
            Fr.e("4546"),
            Fr.e("134523"),
            Fr.e("44356"),
            Fr.e("15623"),
            Fr.e("4566"),
            Fr.e("1223"),
            Fr.e("4546"),
            Fr.e("4256"),
            Fr.e("4456")
        ];

        for (let i=0; i<16; i++) {
            const w = await circuit.calculateWitness({ "selector": i }, true);

            await circuit.checkConstraints(w);

            await circuit.assertOut(w, {out: ct16[i]});
        }
    });

    it("Should create a constant multiplexer 3", async () => {

        const circuit = await wasm_tester(path.join(__dirname, "circuits", "mux3_1.circom"));

        const ct8 = [
            Fr.e("37"),
            Fr.e("47"),
            Fr.e("53"),
            Fr.e("71"),
            Fr.e("89"),
            Fr.e("107"),
            Fr.e("163"),
            Fr.e("191")
        ];

        for (let i=0; i<8; i++) {
            const w = await circuit.calculateWitness({ "selector": i }, true);

            await circuit.checkConstraints(w);

            await circuit.assertOut(w, {out: ct8[i]});
        }
    });
    it("Should create a constant multiplexer 2", async () => {

        const circuit = await wasm_tester(path.join(__dirname, "circuits", "mux2_1.circom"));

        const ct4 = [
            Fr.e("37"),
            Fr.e("47"),
            Fr.e("53"),
            Fr.e("71"),
        ];

        for (let i=0; i<4; i++) {
            const w = await circuit.calculateWitness({ "selector": i }, true);

            await circuit.checkConstraints(w);

            await circuit.assertOut(w, {out: ct4[i]});
        }
    });
    it("Should create a constant multiplexer 1", async () => {

        const circuit = await wasm_tester(path.join(__dirname, "circuits", "mux1_1.circom"));

        const ct2 = [
            Fr.e("37"),
            Fr.e("47"),
        ];

        for (let i=0; i<2; i++) {
            const w = await circuit.calculateWitness({ "selector": i }, true);

            await circuit.checkConstraints(w);

            await circuit.assertOut(w, {out: ct2[i]});
        }
    });
});
