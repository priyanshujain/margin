import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { isMacDesktop } from "./ipc";

if (isMacDesktop) document.documentElement.setAttribute("data-traffic", "");
import "./styles/fonts.css";
import "./styles/tokens.css";
import "./styles/app.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
