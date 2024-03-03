const chai = require("chai");
const assert = chai.assert;

const fs = require("fs");
var tmp = require("tmp-promise");
const path = require("path");
const util = require("util");
const exec = util.promisify(require("child_process").exec);

const BuildZqField = require("./buildzqfield");

module.exports = testField;

async function  testField(prime, test) {
    tmp.setGracefulCleanup();

    const dir = await tmp.dir({prefix: "circom_", unsafeCleanup: true });

    const source = await BuildZqField(prime, "Fr");

    // console.log(dir.path);

    await fs.promises.writeFile(path.join(dir.path, "fr.asm"), source.asm, "utf8");
    await fs.promises.writeFile(path.join(dir.path, "fr.h"), source.h, "utf8");
    await fs.promises.writeFile(path.join(dir.path, "fr.c"), source.c, "utf8");

    await exec(`cp  ${path.join(__dirname,  "tester.cpp")} ${dir.path}`);

    await exec("nasm -fmacho64 --prefix _ " +
               ` ${path.join(dir.path,  "fr.asm")}`
    );

    await exec("g++" +
               ` ${path.join(dir.path,  "tester.cpp")}` +
               ` ${path.join(dir.path,  "fr.o")}` +
               ` ${path.join(dir.path,  "fr.c")}` +
               ` -o ${path.join(dir.path, "tester")}` +
               " -lgmp -g"
    );

    const inLines = [];
    for (let i=0; i<test.length; i++) {
        for (let j=0; j<test[i][0].length; j++) {
            inLines.push(test[i][0][j]);
        }
    }
    inLines.push("");

    await fs.promises.writeFile(path.join(dir.path, "in.tst"), inLines.join("\n"), "utf8");

    await exec(`${path.join(dir.path, "tester")}` +
        ` <${path.join(dir.path, "in.tst")}` +
        ` >${path.join(dir.path, "out.tst")}`);

    const res = await fs.promises.readFile(path.join(dir.path, "out.tst"), "utf8");
    const resLines = res.split("\n");

    for (let i=0; i<test.length; i++) {
        const expected = test[i][1].toString();
        const calculated = resLines[i];

        if (calculated != expected) {
            console.log("FAILED");
            for (let j=0; j<test[i][0].length; j++) {
                console.log(test[i][0][j]);
            }
            console.log("Should Return: " + expected);
            console.log("But Returns: " + calculated);
        }

        assert.equal(calculated, expected);
    }

}

