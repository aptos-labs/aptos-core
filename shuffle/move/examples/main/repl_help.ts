// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

export let overview = {
  "Intro": [
    "This is a Typescript/Javascript repl.",
    'There are 3 top level objects you can use: "main", "devapi", and "helpers".',
  ],

  "main": [
    `"main" is the home of many script functions you can use to interact`,
    `with the modules you've deployed onchain.`,
    `Run "await help.main() for specific information and an example`,
  ],

  "devapi": [
    `"devapi" contains all the information about the account being used.`,
    `The information provided comes from many GET operations of the Diem Dev API.`,
    `Run "await help.devapi() for specific information and an example`,
  ],

  "helpers": [
    `"helpers" holds useful methods which can be used in your development`,
    `Run "await help.helpers() for specific information and an example`,
  ],

  "Try Them Out": [
    `Run each of these commands in the repl to see what functions each has`,
    `ex. Run "devapi". Run "main"`,
  ],

  "Functions vs AsyncFunctions": [
    `Each command provide an arsenal of AsyncFunctions and Functions at your disposal`,
    `Note: To invoke any AsyncFunction, prefix the method with await`,
    `ex. Run "await main.printWelcome()"`,
  ],
};

export async function main(): Promise<any> {
  console.log([
    `"main" is the home of many script functions you can use to interact`,
    `with the modules you've deployed onchain.`,
    `Each script function creates a transaction that is deployed onchain and interacts`,
    `with an associated module that you deployed`,
    `Each of the methods of "main" are found in the project_dir/main/mod.ts.`,
    `Feel free to add more methods in the mod.ts file and use them when running "main"`,
  ]);

  return await ask_for_example([
    `Run "main" in the repl`,
    `Run "await main.setMessageScriptFunction("hello blockchain")"`,
    `This will submit a transaction interacting directly with the`,
    `Message move module which has been deployed.`,
    `To see what your message is onchain, run "await main.decodedMessages()"`,
    `You should get an array with the String "\x00hello blockchain"`,
  ]);
}

export async function devapi(): Promise<any> {
  console.log([
    `"devapi" contains methods that provide information about the account being used.`,
    `The information provided comes from many GET operations of the Diem Dev API.`,
    `Each of the methods of devapi are found in the project_dir/main/devapi.ts.`,
    `With devapi, you can get info on specific accountTransactions, transactions, modules, and resources`,
    `of the account used in the repl.`,
  ]);

  return await ask_for_example([
    `Run "devapi" in the repl`,
    `Run shuffle deploy -p project_path if you haven't already`,
    `This will deploy 3 move modules onchain, specifically "Message", "NFT", and "TestNFT"`,
    `To check if these modules are actually onchain, run await devapi.modules() in the repl.`,
    `You'll find that you will have at least 3 modules with names "Message", "NFT", and "TestNFT.`,
  ]);
}

export async function helpers(): Promise<any> {
  console.log([
    `"helpers" holds useful methods which can be used in your development`,
    `Each of the methods of "helpers" are found in the project_dir/main/helpers.ts.`,
    `With helpers, you are given some useful methods including`,
    `buildAndSubmitTransaction, invokeScriptFunction, etc.`,
  ]);

  return await ask_for_example([
    `Run "helpers" in the repl`,
    `Run "await main.setMessageScriptFunction("hello blockchain")"`,
    `This will submit a transaction interacting directly with the Message`,
    `move module which has been deployed`,
    `Note in the payload of this pending_transaction, there is a field called`,
    `arguments with is a String array.`,
    `This argument "0x68656c6c6f20626c6f636b636861696e" represents the`,
    `hex value of the message that was set`,
    `Run helpers.hexToAscii("0x68656c6c6f20626c6f636b636861696e")`,
    `You should get the output "\x00hello blockchain"`,
  ]);
}

async function ask_for_example(
  example: string[],
  stdin = Deno.stdin,
  stdout = Deno.stdout,
) {
  const buf = new Uint8Array(1024);

  // Write question to console
  await stdout.write(
    new TextEncoder().encode(`Would you like an example? [y/n]: `),
  );

  // Read console's input into answer
  const n = <number> await stdin.read(buf);
  const answer = new TextDecoder().decode(buf.subarray(0, n));

  return delegate_answer(answer.trim(), example);
}

function delegate_answer(answer: String, context: string[]) {
  if (answer != "y" && answer != "n") {
    console.log(`Please enter either "y" or "n"`);
  } else if (answer == "y") {
    console.log(context);
  }
}
