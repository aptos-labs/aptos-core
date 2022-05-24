import { Controller } from "@hotwired/stimulus"

// Connects to data-controller="refresh"
export default class extends Controller {
  static values = {
    src: String,
    interval: {type: Number, default: 5},
  };

  resetTimeout = () => {
    this.timeoutId = setTimeout(() => {
      // Use rAF to delay refresh when tab isn't visible.
      requestAnimationFrame(() => {
        if (this.element.src) {
          this.element.reload();
        } else {
          this.element.src = this.srcValue;
        }
        this.resetTimeout();
      });
    }, this.intervalValue * 1000);
  };

  connect() {
    this.resetTimeout();
  }

  disconnect() {
    clearTimeout(this.timeoutId);
  }
}
