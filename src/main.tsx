import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import App from "./App";
import { inTauri } from "./lib/api";
import "./index.css";

// macOS draws the translucent sidebar material behind the (transparent) window.
if (inTauri && /Mac/.test(navigator.userAgent)) {
  document.documentElement.classList.add("vibrancy");
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
