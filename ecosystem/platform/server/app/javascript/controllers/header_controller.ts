// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Controller } from "./controller";

// Connects to data-controller="header"
export default class extends Controller {
  static targets = ["nav", "navButton", "user", "userButton"];

  declare readonly navTarget: HTMLElement;
  declare readonly navButtonTarget: HTMLElement;
  declare readonly userTarget: HTMLElement;
  declare readonly userButtonTarget: HTMLElement;
  declare readonly hasUserTarget: boolean;

  toggleNav() {
    const open = this.navTarget.toggleAttribute("open");
    for (const icon of this.navButtonTarget.children) {
      icon.classList.toggle("hidden");
    }
    if (open && this.hasUserTarget) this.userTarget.removeAttribute("open");
  }

  toggleUser() {
    const open = this.userTarget.toggleAttribute("open");
    if (open && this.navTarget.hasAttribute("open")) this.toggleNav();
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
    if (!(event.currentTarget instanceof HTMLElement)) return;
    const button = event.currentTarget.querySelector("button");
    if (!(button instanceof HTMLElement)) return;
    if (button.offsetParent == null) return;
    if (
      event.target instanceof HTMLAnchorElement &&
      event.target.parentElement != event.currentTarget
    )
      return;
    button.classList.toggle("rotate-180");
    const navGroup = button.nextElementSibling;
    navGroup?.toggleAttribute("open");
  }

  windowResize(event: Event) {
    if (this.navTarget.hasAttribute("open")) {
      this.toggleNav();
    }
  }

  windowClick(event: Event) {
    // Hide the user dropdown if the user clicks outside.
    if (!(event.target instanceof Element)) return;
    if (!this.hasUserTarget) return;
    if (!this.userTarget.hasAttribute("open")) return;
    if (
      event.target === this.userTarget ||
      this.userTarget.contains(event.target) ||
      event.target === this.userButtonTarget ||
      this.userButtonTarget.contains(event.target)
    ) {
      return;
    }
    this.toggleUser();
  }

  preventDefault(event: Event) {
    event.preventDefault();
  }
}
