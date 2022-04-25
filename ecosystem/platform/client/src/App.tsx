import * as React from "react";
import {Route, Routes} from "react-router";
import {OnboardingPage} from "onboarding";
import {SocialLoginButtonCallbackPage} from "ui";

function App() {
  return (
    <Routes>
      <Route path="/onboarding" element={<OnboardingPage />} />
      <Route path="/oauth" element={<SocialLoginButtonCallbackPage />} />
    </Routes>
  );
}

export default App;
