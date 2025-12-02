import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { DownloadProgress } from "../types";

interface UseModelDownloadProps {
  baseModel: string;
  setIsUncensored: (value: boolean) => void;
  currentModel: string;
  isLlamaAlreadyDownloaded: boolean;
  isModelAlreadyDownloaded: boolean;
  setIsDownloadingLlama: (value: boolean) => void;
  setIsDownloadingModel: (value: boolean) => void;
  setDownloadProgress: (progress: DownloadProgress | null) => void;
  setCurrentToastId: (id: string | number | null) => void;
  addLog: (message: string) => void;
}

interface UseModelDownloadReturn {
  handleDownloadLlama: () => Promise<void>;
  handleDownloadModel: () => Promise<void>;
  handleUncensoredChange: (checked: boolean) => Promise<void>;
}

export const useModelDownload = ({
  baseModel,
  setIsUncensored,
  currentModel,
  isLlamaAlreadyDownloaded,
  isModelAlreadyDownloaded,
  setIsDownloadingLlama,
  setIsDownloadingModel,
  setDownloadProgress,
  setCurrentToastId,
  addLog,
}: UseModelDownloadProps): UseModelDownloadReturn => {
  const handleDownloadLlama = async () => {
    if (isLlamaAlreadyDownloaded) {
      toast.error("Llama already downloaded");
      return;
    }
    setIsDownloadingLlama(true);
    setDownloadProgress(null);

    const toastId = toast.loading("Starting llama.cpp download...");
    setCurrentToastId(toastId);
    addLog("Starting llama.cpp download...");

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
  };

  const handleDownloadModel = async () => {
    if (isModelAlreadyDownloaded) {
      toast.error("Model already downloaded");
      return;
    }

    setIsDownloadingModel(true);
    setDownloadProgress(null);

    const toastId = toast.loading(`Starting model '${currentModel}' download...`);
    setCurrentToastId(toastId);
    addLog(`Starting model '${currentModel}' download...`);

    try {
      const result = await invoke<string>("download_model_by_name", {
        modelName: currentModel,
      });
      toast.success(result, { id: toastId });
      addLog(result);

      // Set as active model after download
      await invoke<string>("set_active_model_command", { modelName: currentModel });
      addLog(`Set active model to: ${currentModel}`);
    } catch (error) {
      toast.error(`Error: ${error}`, { id: toastId });
      addLog(`Error: ${error}`);
    } finally {
      setIsDownloadingModel(false);
      setDownloadProgress(null);
      setCurrentToastId(null);
    }
  };

  const handleUncensoredChange = async (checked: boolean) => {
    setIsUncensored(checked);
    localStorage.setItem("isUncensored", checked.toString());

    const newModelName = checked ? `${baseModel}_uncensored` : baseModel;
    addLog(`Switching to ${checked ? "uncensored" : "censored"} model: ${newModelName}`);

    try {
      // Check if new model is downloaded
      const isDownloaded = await invoke<boolean>("check_model_downloaded", {
        modelName: newModelName,
      });

      if (!isDownloaded) {
        // Model not downloaded, start download
        toast.info(`Model '${newModelName}' not found, starting download...`);
        setIsDownloadingModel(true);
        setDownloadProgress(null);

        const toastId = toast.loading(`Downloading model '${newModelName}'...`);
        setCurrentToastId(toastId);

        try {
          const result = await invoke<string>("download_model_by_name", {
            modelName: newModelName,
          });
          toast.success(result, { id: toastId });
          addLog(result);
        } catch (error) {
          toast.error(`Error: ${error}`, { id: toastId });
          addLog(`Error downloading: ${error}`);
          // Revert checkbox on error
          setIsUncensored(!checked);
          localStorage.setItem("isUncensored", (!checked).toString());
          return;
        } finally {
          setIsDownloadingModel(false);
          setDownloadProgress(null);
          setCurrentToastId(null);
        }
      }

      // Set as active model
      await invoke<string>("set_active_model_command", { modelName: newModelName });
      addLog(`Active model set to: ${newModelName}`);
      toast.success(`Switched to ${checked ? "uncensored" : "censored"} model`);
    } catch (error) {
      toast.error(`Error: ${error}`);
      addLog(`Error switching model: ${error}`);
      // Revert checkbox on error
      setIsUncensored(!checked);
      localStorage.setItem("isUncensored", (!checked).toString());
    }
  };

  return {
    handleDownloadLlama,
    handleDownloadModel,
    handleUncensoredChange,
  };
};

