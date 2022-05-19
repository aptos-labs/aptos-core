import { Controller } from "@hotwired/stimulus"
import { shake } from '../utils';

// Connects to data-controller="shake"
export default class extends Controller {
  static targets = ["content"];

  shake() {
    this.contentTargets.forEach(shake);
  }
}
