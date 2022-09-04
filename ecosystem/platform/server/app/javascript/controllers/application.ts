/*
 * Copyright (c) Aptos
 * SPDX-License-Identifier: Apache-2.0
 */

import { Application } from "@hotwired/stimulus"

declare global {
  interface Window {
    Stimulus: Application;
    aptos?: {
      connect: () => Promise<{address: string, publicKey: string}>;
      signAndSubmitTransaction: (transaction: {}) => Promise<unknown>;
    }
  }
}

const application = Application.start()

// Configure Stimulus development experience
application.debug = false
window.Stimulus   = application

export { application }
