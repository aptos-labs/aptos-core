// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Controller } from "./controller";

// Connects to data-controller="header"
export default class extends Controller {
  static targets = ["nav"];

  declare readonly navTarget: HTMLElement;

  toggleNav() {
    this.navTarget.toggleAttribute('open');
  }
}
