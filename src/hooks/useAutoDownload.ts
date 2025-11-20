import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { BaseDirectory, exists } from "@tauri-apps/plugin-fs";

interface UseAutoDownloadProps {
  modelName: string;
  addLog: (message: string) => void;
  setCurrentToastId: (id: string | number | null) => void;
  setDownloadProgress: (progress: any) => void;
}

export const useAutoDownload = ({
  modelName,
  addLog,
  setCurrentToastId,
  setDownloadProgress,
}: UseAutoDownloadProps) => {
  const [isDownloadingLlama, setIsDownloadingLlama] = useState(false);
  const [isDownloadingModel, setIsDownloadingModel] = useState(false);
  const [isModelAlreadyDownloaded, setIsModelAlreadyDownloaded] = useState(false);
  const [isLlamaAlreadyDownloaded, setIsLlamaAlreadyDownloaded] = useState(false);

  // Check and auto-download required files on startup
  useEffect(() => {
    // Don't run if modelName is not set yet
    if (!modelName) return;

    let hasRun = false;

    const checkAndDownloadFiles = async () => {
      if (hasRun) return; // Prevent double execution
      hasRun = true;

      try {
        let wasSomeDownloads = false;

        // Check if llama-server binary exists
        const llamaBinaryPath = `./bin/llama-server`;
        const llamaExists = await exists(llamaBinaryPath, { baseDir: BaseDirectory.AppData });

        // Check if llama.cpp needs update (even if it exists)
        let needsLlamaUpdate = false;
        if (llamaExists) {
          try {
            needsLlamaUpdate = await invoke<boolean>("check_llama_version");
            if (!needsLlamaUpdate) {
              setIsLlamaAlreadyDownloaded(true);
            }
          } catch (error) {
            console.error("Failed to check llama version:", error);
            needsLlamaUpdate = false;
            setIsLlamaAlreadyDownloaded(true);
          }
        }

        // Check if model exists using backend command
        let modelExists = false;
        try {
          modelExists = await invoke<boolean>("check_model_downloaded", { modelName });
          if (modelExists) {
            setIsModelAlreadyDownloaded(true);
          }
        } catch (error) {
          console.error("Failed to check model:", error);
        }

        // Auto-download llama.cpp if missing or needs update
        if ((!llamaExists || needsLlamaUpdate) && !isDownloadingLlama) {
          wasSomeDownloads = true;
          const message = needsLlamaUpdate 
            ? "llama.cpp update available, downloading new version..."
            : "llama.cpp not found, downloading automatically...";
          addLog(message);
          setIsDownloadingLlama(true);
          setDownloadProgress(null);

          const toastMessage = needsLlamaUpdate
            ? "Updating llama.cpp..."
            : "Starting llama.cpp download...";
          const toastId = toast.loading(toastMessage);
          setCurrentToastId(toastId);

          try {
            const result = await invoke<string>("download_llama_cpp");
            toast.success(result, { id: toastId });
            addLog(result);
          } catch (error) {
            toast.error(`Error: ${error}`, { id: toastId });
            addLog(`Error: ${error}`);
          } finally {
            setIsDownloadingLlama(false);
            setDownloadProgress(null);
            setCurrentToastId(null);
          }
        }

        // Auto-download model if missing
        if (!modelExists && modelName && !isDownloadingModel) {
          wasSomeDownloads = true;
          addLog(`Model '${modelName}' not found, downloading automatically...`);
          setIsDownloadingModel(true);
          setDownloadProgress(null);

          const toastId = toast.loading(`Starting model '${modelName}' download...`);
          setCurrentToastId(toastId);

          try {
            const result = await invoke<string>("download_model_by_name", { modelName });
            toast.success(result, { id: toastId });
            addLog(result);

            // Set as active model after download
            await invoke<string>("set_active_model_command", { modelName });
            addLog(`Active model set to: ${modelName}`);
          } catch (error) {
            toast.error(`Error: ${error}`, { id: toastId });
            addLog(`Error: ${error}`);
          } finally {
            setIsDownloadingModel(false);
            setDownloadProgress(null);
            setCurrentToastId(null);
          }
        }

        if (llamaExists && modelExists && wasSomeDownloads) {
          addLog("All required files are present");
          toast.success("System ready!");
        }
      } catch (error) {
        console.error("Failed to check files:", error);
        addLog(`Failed to check files: ${error}`);
      }
    };

    // Small delay to ensure file system is ready
    const timer = setTimeout(checkAndDownloadFiles, 500);
    return () => clearTimeout(timer);
  }, [modelName]); // eslint-disable-line react-hooks/exhaustive-deps

  return {
    isDownloadingLlama,
    isDownloadingModel,
    isModelAlreadyDownloaded,
    isLlamaAlreadyDownloaded,
    setIsDownloadingLlama,
    setIsDownloadingModel,
  };
};
