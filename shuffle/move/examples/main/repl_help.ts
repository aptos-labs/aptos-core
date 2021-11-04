// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

export const helpObject = [
  "This is a Typescript/Javascript repl, with some helper!",
  "To invoke any AsyncFunction, prefix the method with await",
  "Let's start with the `devapi`.",
  "Try running: `await devapi.transactions();`, `await devapi.modules();`",
  "Now, let's check out `helpers`.",
  "Try running: `await helpers.setMessageScriptFunction('hello');`",
  "Now run `await devapi.accountTransactions();`",
  "To find which arguments must be passed into each method in DiemHelpers look ",
  "at “project_path”/main/helpers.ts",
  "Codegen encapsulates all of the generated typescript from transaction-builders",
  "being run on package main",
  "To find which arguments must be passed into each method in ",
  "codegen look at “project_path”/generated/diemTypes/mod.ts",
];
