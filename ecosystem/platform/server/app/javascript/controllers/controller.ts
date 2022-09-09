import { Controller as BaseController } from "@hotwired/stimulus";

export abstract class Controller<
  ElementType extends Element = Element
> extends BaseController {
  get element() {
    return this.scope.element as ElementType;
  }
}
