import * as React from "react";
import {BrowserRouter as Router} from "react-router-dom";
import {Route, Routes} from "react-router";
import {OnboardingPage} from "onboarding";
import {AuthContextProvider} from "auth";

function App() {
  return (
    <AuthContextProvider>
      <Router>
        <Routes>
          <Route path="/onboarding" element={<OnboardingPage />} />
        </Routes>
      </Router>
    </AuthContextProvider>
  );
}

export default App;
