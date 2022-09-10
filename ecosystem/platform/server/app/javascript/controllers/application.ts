/*
 * Copyright (c) Aptos
 * SPDX-License-Identifier: Apache-2.0
 */

import { Application } from "@hotwired/stimulus";
import type { Types } from "aptos";
import * as Sentry from "@sentry/browser";

declare global {
  interface Window {
    Stimulus: Application;
    aptos?: {
      connect: () => Promise<{ address: string; publicKey: string }>;
      signAndSubmitTransaction: (transaction: {}) => Promise<Types.Transaction>;
      signMessage: (transaction: {}) => Promise<Record<string, unknown>>;
      network: () => Promise<string>;
    };
    martian?: {
      connect: () => Promise<{ address: string; publicKey: string }>;
      account: () => Promise<{ address: string; publicKey: string }>;
      generateSignAndSubmitTransaction: (
        sender: string,
        transaction: {}
      ) => Promise<string>;
      signMessage: (transaction: {}) => Promise<Record<string, unknown>>;
      network: () => Promise<string>;
    };
  }
  var process: { env: Record<string, string | undefined> };
}

const application = Application.start();

// Configure Stimulus development experience
application.debug = false;
window.Stimulus = application;

Sentry.init({
  dsn: process.env.SENTRY_FRONTEND_DSN,
  environment: process.env.NODE_ENV,
  enabled: process.env.NODE_ENV === "production",
});

export { application };
