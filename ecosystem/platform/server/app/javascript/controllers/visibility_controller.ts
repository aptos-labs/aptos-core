// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Controller } from "./controller";

export default class extends Controller<HTMLElement> {
  static targets = ["hideable"];

  declare readonly hideableTargets: HTMLElement[];

  hiddenClass = "hidden";
  storageKey = "hidden_";

  // Show targets on page load if no sessionStorage value

  connect() {
    this.hideableTargets.forEach((el) => {
      let targetHidden: boolean =
        sessionStorage.getItem(this.storageKey + el.id) === "true";
      el.classList.toggle(this.hiddenClass, targetHidden);
    });
  }

  // Show, hide or toggle targets

  showTargets() {
    this.hideableTargets.forEach((el) => {
      el.classList.remove(this.hiddenClass);
    });
  }

  hideTargets() {
    this.hideableTargets.forEach((el) => {
      el.classList.add(this.hiddenClass);
    });
  }

  toggleTargets() {
    this.hideableTargets.forEach((el) => {
      el.classList.toggle(this.hiddenClass);
    });
  }

  // Show, hide or toggle targets with sessionStorage

  showTargetsSessionStorage() {
    this.hideableTargets.forEach((el) => {
      sessionStorage.removeItem(this.storageKey + el.id);
      el.classList.remove(this.hiddenClass);
    });
  }

  hideTargetsSessionStorage() {
    this.hideableTargets.forEach((el) => {
      sessionStorage.setItem(this.storageKey + el.id, JSON.stringify(true));
      el.classList.add(this.hiddenClass);
    });
  }

  toggleTargetsSessionStorage() {
    this.hideableTargets.forEach((el) => {
      el.classList.toggle(this.hiddenClass);
      el.classList.contains(this.hiddenClass)
        ? sessionStorage.setItem(this.storageKey + el.id, JSON.stringify(true))
        : sessionStorage.removeItem(this.storageKey + el.id);
    });
  }
}
