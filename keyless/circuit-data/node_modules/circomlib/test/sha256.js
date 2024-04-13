const chai = require("chai");
const path = require("path");
const crypto = require("crypto");
const F1Field = require("ffjavascript").F1Field;
const Scalar = require("ffjavascript").Scalar;
exports.p = Scalar.fromString("21888242871839275222246405745257275088548364400416034343698204186575808495617");
const Fr = new F1Field(exports.p);

const assert = chai.assert;

const sha256 = require("./helpers/sha256");

const wasm_tester = require("circom_tester").wasm;

// const printSignal = require("./helpers/printsignal");


function buffer2bitArray(b) {
    const res = [];
    for (let i=0; i<b.length; i++) {
        for (let j=0; j<8; j++) {
            res.push((b[i] >> (7-j) &1));
        }
    }
    return res;
}

function bitArray2buffer(a) {
    const len = Math.floor((a.length -1 )/8)+1;
    const b = new Buffer.alloc(len);

    for (let i=0; i<a.length; i++) {
        const p = Math.floor(i/8);
        b[p] = b[p] | (Number(a[i]) << ( 7 - (i%8)  ));
    }
    return b;
}


describe("SHA256 test", function () {
    this.timeout(100000);


    it("Should work bits to array and array to bits", async () => {
        const b = new Buffer.alloc(64);
        for (let i=0; i<64; i++) {
            b[i] = i+1;
        }
        const a = buffer2bitArray(b);
        const b2 = bitArray2buffer(a);

        assert.equal(b.toString("hex"), b2.toString("hex"), true);
    });

    it("Should calculate a hash of 1 compressor", async () => {
        const cir = await wasm_tester(path.join(__dirname, "circuits", "sha256_2_test.circom"));

        const witness = await cir.calculateWitness({ "a": "1", "b": "2" }, true);

        const b = new Buffer.alloc(54);
        b[26] = 1;
        b[53] = 2;

        const hash = crypto.createHash("sha256")
            .update(b)
            .digest("hex");
        const r = "0x" + hash.slice(10);

        const hash2 = sha256.hash(b.toString("hex"), {msgFormat: "hex-bytes"});

        assert.equal(hash, hash2);

        assert(Fr.eq(witness[1], Fr.e(r)));
    }).timeout(1000000);

    it("Should calculate a hash of 2 compressor", async () => {
        const cir = await wasm_tester(path.join(__dirname, "circuits", "sha256_test512.circom"));

        const b = new Buffer.alloc(64);
        for (let i=0; i<64; i++) {
            b[i] = i+1;
        }

        const hash = crypto.createHash("sha256")
            .update(b)
            .digest("hex");

        const arrIn = buffer2bitArray(b);
        const witness = await cir.calculateWitness({ "in": arrIn }, true);

        const arrOut = witness.slice(1, 257);
        const hash2 = bitArray2buffer(arrOut).toString("hex");

        assert.equal(hash, hash2);

    }).timeout(1000000);
    it ("Should calculate a hash of 2 compressor", async () => {
        const cir = await wasm_tester(path.join(__dirname, "circuits", "sha256_test448.circom"));

        const testStr = "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";

        const b = Buffer.from(testStr, "utf8");

        const hash = crypto.createHash("sha256")
            .update(b)
            .digest("hex");

        const arrIn = buffer2bitArray(b);

        const witness = await cir.calculateWitness({ "in": arrIn }, true);

        const arrOut = witness.slice(1, 257);
        const hash2 = bitArray2buffer(arrOut).toString("hex");

        assert.equal(hash, hash2);
    });

});
