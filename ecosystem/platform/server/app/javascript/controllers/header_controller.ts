// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Controller } from "./controller";

// Connects to data-controller="header"
export default class extends Controller {
  static targets = ["nav", "user"];

  declare readonly navTarget: HTMLElement;
  declare readonly userTarget: HTMLElement;
  declare readonly hasUserTarget: boolean;

  toggleNav() {
    const open = this.navTarget.toggleAttribute("open");
    if (open && this.hasUserTarget) this.userTarget.removeAttribute("open");
  }

  toggleUser() {
    const open = this.userTarget.toggleAttribute("open");
    if (open) this.navTarget.removeAttribute("open");
  }

  navGroupHover(event: MouseEvent) {
    // If another nav group has focus (and thus its dropdown is visible),
    // remove focus so that only one dropdown is visible at a time.
    if (!(event.target instanceof HTMLElement)) return;
    if (!(document.activeElement instanceof HTMLElement)) return;
    const hoverGroup = event.target.closest("li.group");
    const focusGroup = document.activeElement.closest("li.group");
    if (focusGroup && focusGroup != hoverGroup) {
      document.activeElement.blur();
    }
  }

  navGroupToggle(event: Event) {
    if (!(event.target instanceof Element)) return;
    const button = event.target?.closest('button');
    if (!(button instanceof HTMLElement)) return;
    button.classList.toggle('rotate-180');
    const navGroup = button.nextElementSibling;
    navGroup?.toggleAttribute("open");
  }
}
