import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles/index.css";
import { TrayIcon } from "@tauri-apps/api/tray";
import { Menu } from "@tauri-apps/api/menu";
import { exit } from "@tauri-apps/plugin-process";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { Image } from "@tauri-apps/api/image";
import { resolveResource } from "@tauri-apps/api/path";

// Initialize tray icon

(async () => {
  const menu = await Menu.new({
    items: [
      {
        id: "open",
        text: "Show Sigma Eclipse LLM",
        action: async () => {
          const window = getCurrentWindow();
          await window.show();
          await window.setFocus();
        },
      },
      {
        id: "quit",
        text: "Quit",
        action: async () => {
          // Stop llama-server before exiting
          try {
            await invoke("stop_server");
          } catch (error) {
            // Ignore errors if server is not running
            console.error("Server stop result:", error);
          }
          exit(0);
        },
      },
    ],
  });

  // Handle menu item clicks

  // Load icon.ico for tray icon
  // Get resource directory and construct path to icon
  const iconFileName = "icon.ico";

  let trayIconImage: Image | string;
  try {
    // Try to load from resources
    const iconPath = await resolveResource(`icons/${iconFileName}`);
    trayIconImage = await Image.fromPath(iconPath);
  } catch (error) {
    console.warn("Failed to load icon from resources, using path string:", error);
    // Fallback to path string
    trayIconImage = `icons/${iconFileName}`;
  }

  const options = {
    icon: trayIconImage,
    menu,
    menuOnLeftClick: true,
  };

  // eslint-disable-next-line no-console
  console.log(options);
  const trayIcon = await TrayIcon.new(options);
  trayIcon.setIconAsTemplate(true);
})();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
