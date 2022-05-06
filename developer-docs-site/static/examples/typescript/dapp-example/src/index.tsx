import React from "react";
import ReactDOM from "react-dom/client";
import "./index.css";
import App from "./App";

// Wait until DOMContentLoaded so that window.aptos is initialized.
document.addEventListener("DOMContentLoaded", () => {
  const root = ReactDOM.createRoot(
    document.getElementById("root") as HTMLElement
  );
  root.render(
    <React.StrictMode>
      <App />
    </React.StrictMode>
  );
});
