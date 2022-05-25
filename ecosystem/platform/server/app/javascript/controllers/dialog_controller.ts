// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Controller } from "./controller";
import dialogPolyfill from "dialog-polyfill";

// Connects to data-controller="dialog"
export default class extends Controller<HTMLDialogElement> {
  connect() {
    dialogPolyfill.registerDialog(this.element);
  }
}
