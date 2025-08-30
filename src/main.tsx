import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./App.css";
import "@carbon/styles/css/styles.css";
import { RecoilRoot } from "recoil";
import { Toaster } from "sonner";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <RecoilRoot>
      <App />
      <Toaster position="bottom-center" />
    </RecoilRoot>
  </React.StrictMode>,
);
