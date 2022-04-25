import * as React from "react";
import {BrowserRouter as Router} from "react-router-dom";
import {Route, Routes} from "react-router";
import {OnboardingPage} from "onboarding";
import {AuthContextProvider} from "auth";
import {SocialLoginButtonCallbackPage} from "ui";

function App() {
  return (
    <AuthContextProvider>
      <Router>
        <Routes>
          <Route path="/onboarding" element={<OnboardingPage />} />
          <Route path="/oauth" element={<SocialLoginButtonCallbackPage />} />
        </Routes>
      </Router>
    </AuthContextProvider>
  );
}

export default App;
