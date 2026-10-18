import React from "react";
import ReactDOM from "react-dom/client";
import "../src/tokens.css";
import "../src/fonts.css";
import Kit from "./Kit";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Kit />
  </React.StrictMode>,
);
