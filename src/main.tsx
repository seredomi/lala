import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./App.css";
import "@carbon/styles/css/styles.css";
import { Toaster } from "sonner";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
    <Toaster position="bottom-center" />
  </React.StrictMode>,
);
