import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AppSettings, RecommendedSettings } from "../types";

interface UseSettingsProps {
  addLog: (message: string) => void;
}

interface UseSettingsReturn {
  baseModel: string;
  setBaseModel: (model: string) => void;
  isUncensored: boolean;
  setIsUncensored: (value: boolean) => void;
  port: number;
  ctxSize: number;
  gpuLayers: number;
  appDataPath: string;
  currentModel: string;
  handleCtxSizeChange: (newCtxSize: number) => Promise<void>;
  handleGpuLayersChange: (newGpuLayers: number) => Promise<void>;
  handleRestoreDefaults: () => Promise<void>;
}

export const useSettings = ({ addLog }: UseSettingsProps): UseSettingsReturn => {
  const [baseModel, setBaseModel] = useState(""); // model or model_s
  const [isUncensored, setIsUncensored] = useState(false);
  const [port, setPort] = useState(10345);
  const [ctxSize, setCtxSize] = useState(6000);
  const [gpuLayers, setGpuLayers] = useState(41);
  const [appDataPath, setAppDataPath] = useState("");

  // Calculate current model name based on base model and uncensored flag
  const currentModel = isUncensored ? `${baseModel}_uncensored` : baseModel;

  // Load settings from backend on mount
  useEffect(() => {
    const loadSettings = async () => {
      try {
        // Load settings from backend (settings.json)
        const settings = await invoke<AppSettings>("get_settings_command");
        // eslint-disable-next-line no-console
        console.log("Loaded settings:", settings);

        // Extract base model from active_model (remove _uncensored suffix if present)
        const isUncensoredModel = settings.active_model.endsWith("_uncensored");
        const baseModelName = isUncensoredModel
          ? settings.active_model.replace("_uncensored", "")
          : settings.active_model;

        // Also check localStorage for uncensored preference (UI state)
        const savedUncensored = localStorage.getItem("isUncensored");
        const uncensored = savedUncensored === "true" || isUncensoredModel;

        setBaseModel(baseModelName);
        setIsUncensored(uncensored);
        setPort(settings.port);
        setCtxSize(settings.ctx_size);
        setGpuLayers(settings.gpu_layers);

        addLog(
          `Settings loaded: port=${settings.port}, ctx_size=${settings.ctx_size}, gpu_layers=${settings.gpu_layers}`
        );
        addLog(`Active model: ${settings.active_model}`);

        // Get recommended settings for memory info
        try {
          const recommended = await invoke<RecommendedSettings>("get_recommended_settings");
          addLog(`System RAM: ${recommended.memory_gb} GB`);
        } catch {
          // Ignore
        }

        // Check if the model is downloaded, if not try base model
        try {
          const currentModelName = uncensored ? `${baseModelName}_uncensored` : baseModelName;
          const isDownloaded = await invoke<boolean>("check_model_downloaded", {
            modelName: currentModelName,
          });

          if (isDownloaded) {
            await invoke<string>("set_active_model_command", { modelName: currentModelName });
          } else {
            // If preferred model not downloaded, try base model
            const baseModelDownloaded = await invoke<boolean>("check_model_downloaded", {
              modelName: baseModelName,
            });

            if (baseModelDownloaded) {
              await invoke<string>("set_active_model_command", { modelName: baseModelName });
              addLog(`Active model set to: ${baseModelName}`);
            }
          }
        } catch (error) {
          console.error("Failed to set active model:", error);
        }
      } catch (error) {
        console.error("Failed to load settings:", error);
        // Fallback to smaller model if detection fails
        setBaseModel("model_s");
        addLog("Failed to load settings, using defaults");
      }
    };

    loadSettings();
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Get app data path on mount
  useEffect(() => {
    invoke<string>("get_app_data_path")
      .then((path) => setAppDataPath(path))
      .catch((error) => console.error("Failed to get app data path:", error));
  }, []);

  const handleCtxSizeChange = async (newCtxSize: number) => {
    setCtxSize(newCtxSize);
    try {
      await invoke<string>("set_ctx_size_command", { ctxSize: newCtxSize });
    } catch (error) {
      console.error("Failed to save ctx_size:", error);
    }
  };

  const handleGpuLayersChange = async (newGpuLayers: number) => {
    setGpuLayers(newGpuLayers);
    try {
      await invoke<string>("set_gpu_layers_command", { gpuLayers: newGpuLayers });
    } catch (error) {
      console.error("Failed to save gpu_layers:", error);
    }
  };

  const handleRestoreDefaults = async () => {
    try {
      const recommended = await invoke<RecommendedSettings>("get_recommended_settings");

      // Apply recommended settings
      setCtxSize(recommended.recommended_ctx_size);
      setGpuLayers(recommended.recommended_gpu_layers);

      // Save to backend
      await invoke<string>("set_ctx_size_command", { ctxSize: recommended.recommended_ctx_size });
      await invoke<string>("set_gpu_layers_command", {
        gpuLayers: recommended.recommended_gpu_layers,
      });

      addLog(
        `Settings restored: ctx_size=${recommended.recommended_ctx_size}, gpu_layers=${recommended.recommended_gpu_layers}`
      );
    } catch (error) {
      console.error("Failed to restore defaults:", error);
      throw error; // Re-throw for caller to handle toast
    }
  };

  return {
    baseModel,
    setBaseModel,
    isUncensored,
    setIsUncensored,
    port,
    ctxSize,
    gpuLayers,
    appDataPath,
    currentModel,
    handleCtxSizeChange,
    handleGpuLayersChange,
    handleRestoreDefaults,
  };
};


