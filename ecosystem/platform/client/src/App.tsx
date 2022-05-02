// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {OnboardingPage} from "onboarding";
import * as React from "react";
import {Route, Routes} from "react-router";
import {SocialLoginButtonCallbackPage} from "ui";

export function App() {
  return (
    <Routes>
      <Route path="/onboarding" element={<OnboardingPage />} />
      <Route path="/oauth" element={<SocialLoginButtonCallbackPage />} />
    </Routes>
  );
}
