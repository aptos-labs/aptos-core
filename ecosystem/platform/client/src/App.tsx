import * as React from "react";
import {BrowserRouter as Router} from "react-router-dom";
import {Route, Routes} from "react-router";
import {OnboardingPage} from "onboarding";

function App() {
  return (
    <Router>
      <Routes>
        <Route path="/onboarding" element={<OnboardingPage />} />
      </Routes>
    </Router>
  );
}

export default App;
