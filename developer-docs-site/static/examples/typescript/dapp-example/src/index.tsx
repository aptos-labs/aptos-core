import React from "react";
import ReactDOM from "react-dom/client";
import "./index.css";
import App from "./App";

declare global {
  interface Window {
    aptos: any;
  }
}

const root = ReactDOM.createRoot(
  document.getElementById("root") as HTMLElement
);

window.addEventListener("load", () => {
  root.render(
    <React.StrictMode>
      <App />
    </React.StrictMode>
  );
});
