import { Controller } from "./controller";

export default class extends Controller<HTMLTableRowElement> {
  tableRowClick(event: Event) {
    if (!(event.currentTarget instanceof Element) || event.defaultPrevented)
      return;

    const anchor = event.currentTarget.querySelector("a");
    if (!anchor) return;

    if (event.target !== anchor) {
      anchor?.click();
    }
  }
}
