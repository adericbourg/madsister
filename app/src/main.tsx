import React from "react";
import ReactDOM from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import App from "./App";
import { batchPrint, type PrintPlan } from "./ui/batchPrint";
import "@fontsource/patrick-hand/latin-400.css";
import "@fontsource/kalam/latin-400.css";
import "@fontsource/kalam/latin-700.css";
import "./app.css";

const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement);
invoke<PrintPlan | null>("print_plan").then((plan) =>
  plan
    ? batchPrint(plan)
    : root.render(
        <React.StrictMode>
          <App />
        </React.StrictMode>,
      ),
);
