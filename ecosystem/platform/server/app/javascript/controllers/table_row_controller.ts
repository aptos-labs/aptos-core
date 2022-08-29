import { Controller } from "./controller";

export default class extends Controller<HTMLTableRowElement> {

  tableRowClick(event: Event) {
    if (!(event.currentTarget instanceof Element)) return;
    const validatorAddress = event.currentTarget.querySelector('a');
    if (event.target !== validatorAddress) {
      event.preventDefault();
      validatorAddress?.click();
    }
  }
}
