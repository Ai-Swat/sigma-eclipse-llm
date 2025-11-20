import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { BaseDirectory, exists } from "@tauri-apps/plugin-fs";

interface UseAutoDownloadProps {
  modelUrl: string;
  addLog: (message: string) => void;
  setCurrentToastId: (id: string | number | null) => void;
  setDownloadProgress: (progress: any) => void;
}

export const useAutoDownload = ({
  modelUrl,
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
    // Don't run if modelUrl is not set yet
    if (!modelUrl) return;

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

        // Check if model exists
        const modelPath = `./models/model.gguf`;
        const modelExists = await exists(modelPath, { baseDir: BaseDirectory.AppData });

        if (modelExists) {
          setIsModelAlreadyDownloaded(true);
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

        // Auto-download model if missing and we have a URL
        if (!modelExists && modelUrl.trim() && !isDownloadingModel) {
          wasSomeDownloads = true;
          addLog("Model not found, downloading automatically...");
          setIsDownloadingModel(true);
          setDownloadProgress(null);

          const toastId = toast.loading(`Starting model download...`);
          setCurrentToastId(toastId);

          try {
            const result = await invoke<string>("download_model", { modelUrl });
            toast.success(result, { id: toastId });
            addLog(result);
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
  }, [modelUrl]); // eslint-disable-line react-hooks/exhaustive-deps

  return {
    isDownloadingLlama,
    isDownloadingModel,
    isModelAlreadyDownloaded,
    isLlamaAlreadyDownloaded,
    setIsDownloadingLlama,
    setIsDownloadingModel,
  };
};
