import { Controller } from "@hotwired/stimulus";
import { shake } from "../utils";

// Connects to data-controller="shake"
export default class extends Controller {
  static targets = ["content"];

  declare readonly contentTargets: Element[];

  shake() {
    this.contentTargets.forEach(shake);
  }
}
