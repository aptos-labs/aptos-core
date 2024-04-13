const chai = require("chai");
const path = require("path");

const createBlakeHash = require("blake-hash");
const buildEddsa = require("circomlibjs").buildEddsa;

const assert = chai.assert;

const wasm_tester = require("circom_tester").wasm;
const utils = require("ffjavascript").utils;
const Scalar = require("ffjavascript").Scalar;

describe("Baby Jub test", function () {
    let eddsa;
    let F;
    let circuitAdd;
    let circuitTest;
    let circuitPbk;

    this.timeout(100000);

    before( async() => {

        eddsa = await buildEddsa();
        F = eddsa.F;

        circuitAdd = await wasm_tester(path.join(__dirname, "circuits", "babyadd_tester.circom"));

        circuitTest = await wasm_tester(path.join(__dirname, "circuits", "babycheck_test.circom"));

        circuitPbk = await wasm_tester(path.join(__dirname, "circuits", "babypbk_test.circom"));
    });

    it("Should add point (0,1) and (0,1)", async () => {

        const input={
            x1: 0,
            y1: 1,
            x2: 0,
            y2: 1
        };

        const w = await circuitAdd.calculateWitness(input, true);

        await circuitAdd.assertOut(w, {xout: 0, yout: 1});
    });

    it("Should add 2 same numbers", async () => {

        const input={
            x1: 17777552123799933955779906779655732241715742912184938656739573121738514868268n,
            y1: 2626589144620713026669568689430873010625803728049924121243784502389097019475n,
            x2: 17777552123799933955779906779655732241715742912184938656739573121738514868268n,
            y2: 2626589144620713026669568689430873010625803728049924121243784502389097019475n
        };

        const w = await circuitAdd.calculateWitness(input, true);

        await circuitAdd.assertOut(w, {
            xout: 6890855772600357754907169075114257697580319025794532037257385534741338397365n,
            yout: 4338620300185947561074059802482547481416142213883829469920100239455078257889n
        });

    });

    it("Should add 2 different numbers", async () => {

        const input={
            x1: 17777552123799933955779906779655732241715742912184938656739573121738514868268n,
            y1: 2626589144620713026669568689430873010625803728049924121243784502389097019475n,
            x2: 16540640123574156134436876038791482806971768689494387082833631921987005038935n,
            y2: 20819045374670962167435360035096875258406992893633759881276124905556507972311n
        };

        const w = await circuitAdd.calculateWitness(input, true);

        await circuitAdd.assertOut(w, {
            xout: 7916061937171219682591368294088513039687205273691143098332585753343424131937n,
            yout: 14035240266687799601661095864649209771790948434046947201833777492504781204499n
        });

    });

    it("Should check (0,1) is a valid point", async() => {
        const w = await circuitTest.calculateWitness({x: 0, y:1}, true);

        await circuitTest.checkConstraints(w);
    });

    it("Should check (1,0) is an invalid point", async() => {
        try {
            await circuitTest.calculateWitness({x: 1, y: 0}, true);
            assert(false, "Should be a valid point");
        } catch(err) {
            assert(err.message.includes("Assert Failed"));
        }
    });

    it("Should extract the public key from the private one", async () => {

        const rawpvk = Buffer.from("0001020304050607080900010203040506070809000102030405060708090021", "hex");
        const pvk    = eddsa.pruneBuffer(createBlakeHash("blake512").update(rawpvk).digest().slice(0,32));
        const S      = Scalar.shr(utils.leBuff2int(pvk), 3);

        const A      = eddsa.prv2pub(rawpvk);

        const input = {
            in : S
        };

        const w = await circuitPbk.calculateWitness(input, true);

        await circuitPbk.assertOut(w, {Ax : F.toObject(A[0]), Ay: F.toObject(A[1])});

        await circuitPbk.checkConstraints(w);
    });

});
