const chai = require("chai");
const assert = chai.assert;

const fs = require("fs");
var tmp = require("tmp-promise");
const path = require("path");
const util = require("util");
const exec = util.promisify(require("child_process").exec);

const bigInt = require("big-integer");
const BuildZqField = require("./buildzqfield");
const ZqField = require("fflib").ZqField;

module.exports = testField;

function toMontgomeryStr(a, prime) {
    const n64 = Math.floor((prime.bitLength() - 1) / 64)+1;
    return a.shiftLeft(n64*64).mod(prime).toString(10);
}

function fromMontgomeryStr(a, prime) {
    const n64 = Math.floor((prime.bitLength() - 1) / 64)+1;
    const R = bigInt.one.shiftLeft(n64*64).mod(prime);
    const RI = R.modInv(prime);
    return bigInt(a).times(RI).mod(prime);
}


async function  testField(prime, test) {
    tmp.setGracefulCleanup();

    const F = new ZqField(prime);

    const dir = await tmp.dir({prefix: "circom_", unsafeCleanup: true });

    const [hSource, cSource] = BuildZqField(prime, "Fr");

    await fs.promises.writeFile(path.join(dir.path, "fr.h"), hSource, "utf8");
    await fs.promises.writeFile(path.join(dir.path, "fr.c"), cSource, "utf8");

    await exec("g++" +
               ` ${path.join(__dirname,  "tester.c")}` +
               ` ${path.join(dir.path,  "fr.c")}` +
               ` -o ${path.join(dir.path, "tester")}` +
               " -lgmp"
    );

    for (let i=0; i<test.length; i++) {
        let a = bigInt(test[i][1]).mod(prime);
        if (a.isNegative()) a = prime.add(a);
        let b = bigInt(test[i][2]).mod(prime);
        if (b.isNegative()) b = prime.add(b);
        const ec = F[test[i][0]](a,b);
        // console.log(toMontgomeryStr(a, prime));
        // console.log(toMontgomeryStr(b, prime));
        const res = await exec(`${path.join(dir.path, "tester")}` +
            ` ${test[i][0]}` +
            ` ${toMontgomeryStr(a, prime)}` +
            ` ${toMontgomeryStr(b, prime)}`
        );
        // console.log(res.stdout);
        const c=fromMontgomeryStr(res.stdout, prime);

        assert.equal(ec.toString(), c.toString());
    }

}

