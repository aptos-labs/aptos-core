/*
 * Copyright (c) Aptos
 * SPDX-License-Identifier: Apache-2.0
 */

import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  connect() {
    this.element.textContent = "Hello World!"
  }
}
