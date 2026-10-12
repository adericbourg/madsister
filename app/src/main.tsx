import React from "react";
import ReactDOM from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import App from "./App";
import { BatchPrint, type PrintPlan } from "./ui/BatchPrint";
import "@fontsource/patrick-hand/latin-400.css";
import "@fontsource/kalam/latin-400.css";
import "@fontsource/kalam/latin-700.css";
import "./app.css";

const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement);
invoke<PrintPlan | null>("print_plan").then((plan) =>
  // Not in StrictMode: its doubled effects would print each song twice.
  root.render(
    plan ? (
      <BatchPrint plan={plan} />
    ) : (
      <React.StrictMode>
        <App />
      </React.StrictMode>
    ),
  ),
);
