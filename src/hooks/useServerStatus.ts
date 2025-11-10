import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ServerStatus } from "../types";

export const useServerStatus = () => {
  const [status, setStatus] = useState<ServerStatus>({
    is_running: false,
    message: "Not running",
  });

  // Check server status periodically
  useEffect(() => {
    const interval = setInterval(async () => {
      try {
        const status = await invoke<ServerStatus>("get_server_status");
        setStatus(status);
      } catch (error) {
        console.error("Failed to get status:", error);
      }
    }, 2000);

    return () => clearInterval(interval);
  }, []);

  return status;
};

