import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { ServerStatus } from "../types";

interface UseServerControlProps {
  addLog: (message: string) => void;
  port: number;
  ctxSize: number;
  gpuLayers: number;
  status: ServerStatus;
}

interface UseServerControlReturn {
  isBusy: boolean;
  handleStartServer: () => Promise<void>;
  handleStopServer: () => Promise<void>;
  handleClearAllData: () => Promise<void>;
}

export const useServerControl = ({
  addLog,
  port,
  ctxSize,
  gpuLayers,
  status,
}: UseServerControlProps): UseServerControlReturn => {
  const [isBusy, setIsBusy] = useState(false);

  const handleStartServer = async () => {
    if (isBusy) return;
    setIsBusy(true);

    addLog(`Starting LLM on port ${port} (ctx: ${ctxSize}, gpu layers: ${gpuLayers})...`);
    try {
      // Server now reads settings from settings.json
      const result = await invoke<string>("start_server");
      toast.success(result);
      addLog(result);
    } catch (error) {
      toast.error(`Error: ${error}`);
      addLog(`Error: ${error}`);
    } finally {
      setIsBusy(false);
    }
  };

  const handleStopServer = async () => {
    if (isBusy) return;
    setIsBusy(true);

    addLog("Stopping server...");
    try {
      const result = await invoke<string>("stop_server");
      toast.success(result);
      addLog(result);
    } catch (error) {
      toast.error(`Error: ${error}`);
      addLog(`Error: ${error}`);
    } finally {
      setIsBusy(false);
    }
  };

  const handleClearAllData = async () => {
    const toastId = toast.loading("Preparing to clear all data...");

    try {
      // Stop server first if it's running
      if (status.is_running) {
        addLog("Stopping server before clearing data...");
        toast.loading("Stopping server first...", { id: toastId });

        try {
          await invoke<string>("stop_server");
          addLog("Server stopped");
        } catch (error) {
          addLog(`Warning: Failed to stop server: ${error}`);
        }

        // Wait a bit for server to fully stop
        await new Promise((resolve) => setTimeout(resolve, 500));
      }

      // Now clear all data
      addLog("Clearing all data...");
      toast.loading("Clearing all data...", { id: toastId });

      const result = await invoke<string>("clear_all_data");
      toast.success(result, { id: toastId });
      addLog(result);
    } catch (error) {
      toast.error(`Error: ${error}`, { id: toastId });
      addLog(`Error: ${error}`);
    }
  };

  return {
    isBusy,
    handleStartServer,
    handleStopServer,
    handleClearAllData,
  };
};


