// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { docopt } from "docopt";
import globby from "globby";
import fs from "fs";
import { BCS, HexString } from "aptos";
import { pascalCase } from "change-case";
import { ScriptFunctionABI, ScriptABI } from "aptos/dist/transaction_builder/aptos_types";
import { MoveModule, EntryFunction } from "./ir";

const doc = `
Usage:
  code-gen <abi_path>
  code-gen -h | --help | --version
`;

async function genTSCode(abiPath: string, moduleMap: Map<string, MoveModule>) {
  const buf = await fs.promises.readFile(abiPath);
  const deserializer = new BCS.Deserializer(Uint8Array.from(buf));
  const scriptABI = ScriptABI.deserialize(deserializer);
  if (scriptABI instanceof ScriptFunctionABI) {
    const entryFunc = new EntryFunction(scriptABI);

    const moduleAddress = HexString.fromUint8Array(scriptABI.module_name.address.address);
    const moduleName = scriptABI.module_name.name.value;

    const fullModuleName = `${moduleAddress}::${moduleName}`;

    if (!moduleMap.has(fullModuleName)) {
      moduleMap.set(fullModuleName, new MoveModule(moduleAddress, moduleName));
    }

    moduleMap.get(fullModuleName).addEntryFunction(entryFunc);
  }
}

async function run() {
  const args = docopt(doc);

  const moduleMap = new Map<string, MoveModule>();

  const paths = await globby(`${args["<abi_path>"]}/**/*.abi`);

  await Promise.all(paths.map((path) => genTSCode(path, moduleMap)));

  // TODO: remove hardcoded path
  const dir = "./artifacts";
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }
  moduleMap.forEach((m, k) => {
    const moduleName = k.split("::")[1];
    fs.writeFileSync(`${dir}/${pascalCase(moduleName)}.ts`, m.gen().join("\n"));
  });
}

run();
