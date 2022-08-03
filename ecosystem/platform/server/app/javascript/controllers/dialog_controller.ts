// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Controller } from "./controller";

// Connects to data-controller="dialog"
export default class extends Controller<HTMLDialogElement> {
  handleClick(e: Event) {
    if (e.target === this.element) {
      this.element.close();
    }
  }
}
