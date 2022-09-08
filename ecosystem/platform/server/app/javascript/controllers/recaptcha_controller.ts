import { Controller } from "@hotwired/stimulus";
import { shake } from "../utils";

interface Grecaptcha {
  getResponse: (widgetId?: string) => string;
}

declare global {
  interface Window {
    grecaptcha: Grecaptcha;
  }
}

// Connects to data-controller="recaptcha"
export default class extends Controller {
  validate(event: SubmitEvent) {
    const recaptchav3 = document.getElementsByClassName("grecaptcha-badge")[0];
    if (recaptchav3) return true;

    const response = window.grecaptcha.getResponse();
    if (response.length === 0) {
      event.preventDefault();
      const element = document.getElementsByClassName("g-recaptcha")[0];
      shake(element);
      return false;
    } else {
      return true;
    }
  }
}
