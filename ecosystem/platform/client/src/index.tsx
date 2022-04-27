// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from "react";
import ReactDOM from "react-dom/client";
import {BrowserRouter as Router} from "react-router-dom";
import "./index.css";
import App from "./App";
import {AuthProvider} from "auth";
import * as Sentry from "@sentry/react";
import {BrowserTracing} from "@sentry/tracing";

import ReactGA from "react-ga4";

ReactGA.initialize(process.env.REACT_APP_GA_MEASUREMENT_ID!, {
  testMode: process.env.NODE_ENV !== "production",
});

Sentry.init({
  dsn: process.env.REACT_APP_SENTRY_DSN,
  integrations: [new BrowserTracing()],
  environment: process.env.NODE_ENV,
  enabled: process.env.NODE_ENV === "production",

  // Set tracesSampleRate to 1.0 to capture 100%
  // of transactions for performance monitoring.
  // We recommend adjusting this value in production
  tracesSampleRate: 0.5,
});

const root = ReactDOM.createRoot(
  document.getElementById("root") as HTMLElement,
);

root.render(
  <React.StrictMode>
    <AuthProvider>
      <Router>
        <App />
      </Router>
    </AuthProvider>
  </React.StrictMode>,
);
