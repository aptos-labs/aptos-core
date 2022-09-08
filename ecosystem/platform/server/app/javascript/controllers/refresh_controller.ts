import { Controller } from "./controller";
import type { FrameElement } from "@hotwired/turbo/dist/types/elements";

// Connects to data-controller="refresh"
export default class extends Controller<FrameElement> {
  static values = {
    src: String,
    interval: { type: Number, default: 5 },
  };

  timeoutId: number | undefined;
  declare readonly srcValue: string;
  declare readonly intervalValue: number;

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
