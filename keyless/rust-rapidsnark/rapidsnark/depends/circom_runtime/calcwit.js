#!/usr/bin/env node
/*

Copyright 2020 0KIMS association.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

*/

import fs from "fs";
const pkg = JSON.parse(fs.readFileSync("./package.json"));
const version = pkg.version;

import WitnessCalculatorBuilder from "./js/witness_calculator.js";
import { utils } from "ffjavascript";
import yargs from "yargs";



const argv = yargs
    .version(version)
    .usage("calcwit -w [wasm file] -i [input file JSON] -o [output ouput file file  .json|.bin]")
    .alias("o", "output")
    .alias("i", "input")
    .alias("w", "wasm")
    .help("h")
    .alias("h", "help")
    .epilogue(`Copyright (C) 2018  0kims association
    This program comes with ABSOLUTELY NO WARRANTY;
    This is free software, and you are welcome to redistribute it
    under certain conditions; see the COPYING file in the official
    repo directory at  https://github.com/iden3/circom `)
    .argv;

const inputFileName = typeof(argv.input) === "string" ?  argv.input : "input.json";
const outputFileName = typeof(argv.output) === "string" ?  argv.output : "witness.bin";
const wasmFileName = typeof(argv.wasm) === "string" ?  argv.wasm : "circuit.wasm";


async function run() {


    const input = utils.unstringifyBigInts(JSON.parse(await fs.promises.readFile(inputFileName, "utf8")));
    const wasm = await fs.promises.readFile(wasmFileName);

    const wc = await WitnessCalculatorBuilder(wasm);

    const outputExtension = outputFileName.split(".").pop();

    if (outputExtension === "json") {
        const w = await wc.calculateWitness(input);

        await fs.promises.writeFile(outputFileName, JSON.stringify(utils.stringifyBigInts(w), null, 1));

    } else {
        const w = await wc.calculateBinWitness(input);

        var wstream = fs.createWriteStream(outputFileName);

        wstream.write(Buffer.from(w));
        wstream.end();
        await new Promise(fulfill => wstream.on("finish", fulfill));
    }


}


run();
