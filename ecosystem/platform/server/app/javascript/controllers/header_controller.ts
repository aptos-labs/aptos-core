// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Controller } from "./controller";

// Connects to data-controller="header"
export default class extends Controller {
  static targets = ["nav", "user"];

  declare readonly navTarget: HTMLElement;
  declare readonly userTarget: HTMLElement;

  toggleNav() {
    const open = this.navTarget.toggleAttribute('open');
    if (open) this.userTarget.removeAttribute('open');
  }

  toggleUser() {
    const open = this.userTarget.toggleAttribute('open');
    if (open) this.navTarget.removeAttribute('open');
  }
}
