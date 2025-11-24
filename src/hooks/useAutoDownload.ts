import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";

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
  
  // Use ref to track if download is already in progress (persists between renders)
  const llamaDownloadInProgress = useRef(false);
  const modelDownloadInProgress = useRef(false);
  const processedModels = useRef(new Set<string>());

  // Check and auto-download required files on startup
  useEffect(() => {
    // Don't run if modelName is not set yet
    if (!modelName) return;

    // Prevent processing the same model multiple times
    if (processedModels.current.has(modelName)) return;

    const checkAndDownloadFiles = async () => {
      // Mark this model as being processed
      processedModels.current.add(modelName);

      try {
        let wasSomeDownloads = false;

        // Check if llama.cpp needs update using backend command (works cross-platform)
        let needsLlamaUpdate = false;
        let llamaExists = false;
        
        try {
          needsLlamaUpdate = await invoke<boolean>("check_llama_version");
          // If check_llama_version returns false, llama exists and is up to date
          llamaExists = true;
          if (!needsLlamaUpdate) {
            setIsLlamaAlreadyDownloaded(true);
          }
        } catch (error) {
          // If check fails, assume llama doesn't exist
          console.log("llama.cpp not found or check failed, will download");
          needsLlamaUpdate = true;
          llamaExists = false;
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
        if ((!llamaExists || needsLlamaUpdate) && !llamaDownloadInProgress.current) {
          wasSomeDownloads = true;
          llamaDownloadInProgress.current = true; // Set ref immediately to prevent race condition
          
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
            llamaDownloadInProgress.current = false; // Reset ref after completion
          }
        }

        // Auto-download model if missing
        if (!modelExists && modelName && !modelDownloadInProgress.current) {
          wasSomeDownloads = true;
          modelDownloadInProgress.current = true; // Set ref immediately to prevent race condition
          
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
            modelDownloadInProgress.current = false; // Reset ref after completion
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
