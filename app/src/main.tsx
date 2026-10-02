import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "@fontsource/patrick-hand/latin-400.css";
import "@fontsource/kalam/latin-400.css";
import "@fontsource/kalam/latin-700.css";
import "./app.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
