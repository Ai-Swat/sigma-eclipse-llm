import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Toaster, toast } from "sonner";
import {
  HeaderSection,
  SettingsPanel,
  StatusPanel,
  LogsSection,
  ThemeSwitcher,
} from "./components";
import { useTheme, useServerStatus, useLogs, useDownloadProgress, useAutoDownload } from "./hooks";
import { AppSettings, RecommendedSettings } from "./types";
import "./App.css";

function App() {
  const { theme, toggleTheme } = useTheme();
  const status = useServerStatus();
  const { logs, addLog } = useLogs();
  const isProduction = import.meta.env.PROD;

  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [baseModel, setBaseModel] = useState(""); // model or model_s
  const [isUncensored, setIsUncensored] = useState(false);
  const [port, setPort] = useState(10345);
  const [ctxSize, setCtxSize] = useState(6000);
  const [gpuLayers, setGpuLayers] = useState(41);
  const [appDataPath, setAppDataPath] = useState("");

  const { downloadProgress, setCurrentToastId, setDownloadProgress } = useDownloadProgress(addLog);

  // Calculate current model name based on base model and uncensored flag
  const currentModel = isUncensored ? `${baseModel}_uncensored` : baseModel;

  const {
    isDownloadingLlama,
    isDownloadingModel,
    isModelAlreadyDownloaded,
    isLlamaAlreadyDownloaded,
    setIsDownloadingLlama,
    setIsDownloadingModel,
  } = useAutoDownload({
    modelName: currentModel,
    addLog,
    setCurrentToastId,
    setDownloadProgress,
  });

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
      toast.success("Settings restored to defaults");
    } catch (error) {
      console.error("Failed to restore defaults:", error);
      toast.error(`Error restoring defaults: ${error}`);
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

  return (
    <main className="container">
      <HeaderSection onToggleSettings={() => setIsSettingsOpen(!isSettingsOpen)} />

      <SettingsPanel
        isOpen={isSettingsOpen}
        appDataPath={appDataPath}
        baseModel={baseModel}
        isUncensored={isUncensored}
        ctxSize={ctxSize}
        gpuLayers={gpuLayers}
        isDownloadingLlama={isDownloadingLlama}
        isDownloadingModel={isDownloadingModel}
        downloadProgress={downloadProgress}
        status={status}
        onClose={() => setIsSettingsOpen(false)}
        onDownloadLlama={handleDownloadLlama}
        onDownloadModel={handleDownloadModel}
        onUncensoredChange={handleUncensoredChange}
        onCtxSizeChange={handleCtxSizeChange}
        onGpuLayersChange={handleGpuLayersChange}
        onRestoreDefaults={handleRestoreDefaults}
        onClearAllData={handleClearAllData}
        isProduction={isProduction}
      />

      <div className="content">
        <StatusPanel
          status={status}
          onStartServer={handleStartServer}
          onStopServer={handleStopServer}
          isBusy={isBusy}
        />

        {!isProduction && <LogsSection logs={logs} />}

        <div className="footer-section">
          <ThemeSwitcher theme={theme} onToggleTheme={toggleTheme} />
        </div>
      </div>

      <Toaster position="bottom-right" expand={true} richColors closeButton dir="ltr" />
    </main>
  );
}

export default App;
