import React from "react";
import ReactDOM from "react-dom/client";

import App from "./App";
import { initNotifications } from "./features/notifications/stores/notifications";
import { installCloseHandler } from "./lib/app-close";
import { initTheme } from "./stores/theme";
import "./index.css";

initTheme();
installCloseHandler();
initNotifications();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
