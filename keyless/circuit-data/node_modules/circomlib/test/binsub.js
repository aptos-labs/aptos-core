const path = require("path");

const Scalar = require("ffjavascript").Scalar;
const wasm_tester = require("circom_tester").wasm;

function print(circuit, w, s) {
    console.log(s + ": " + w[circuit.getSignalIdx(s)]);
}

async function checkSub(_a,_b, circuit) {
    let a=Scalar.e(_a);
    let b=Scalar.e(_b);
    if (Scalar.lt(a, 0)) a = Scalar.add(a, Scalar.shl(1, 16));
    if (Scalar.lt(b, 0)) b = Scalar.add(b, Scalar.shl(1, 16));
    const w = await circuit.calculateWitness({a: a, b: b}, true);

    let res = Scalar.sub(a, b);
    if (Scalar.lt(res, 0)) res = Scalar.add(res, Scalar.shl(1, 16));

    await circuit.assertOut(w, {out: res});
}

describe("BinSub test", function () {

    this.timeout(100000);

    let circuit;
    before( async() => {
        circuit = await wasm_tester(path.join(__dirname, "circuits", "binsub_test.circom"));
    });

    it("Should check variuos ege cases", async () => {
        await checkSub(0,0, circuit);
        await checkSub(1,0, circuit);
        await checkSub(-1,0, circuit);
        await checkSub(2,1, circuit);
        await checkSub(2,2, circuit);
        await checkSub(2,3, circuit);
        await checkSub(2,-1, circuit);
        await checkSub(2,-2, circuit);
        await checkSub(2,-3, circuit);
        await checkSub(-2,-3, circuit);
        await checkSub(-2,-2, circuit);
        await checkSub(-2,-1, circuit);
        await checkSub(-2,0, circuit);
        await checkSub(-2,1, circuit);
        await checkSub(-2,2, circuit);
        await checkSub(-2,3, circuit);
    });


});
