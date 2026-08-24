import React from "react";
import ReactDOM from "react-dom/client";
import ProductionApp from "./ProductionApp";
import "./commercial.css";
import "./production.css";
import "./network-shield.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ProductionApp />
  </React.StrictMode>,
);
