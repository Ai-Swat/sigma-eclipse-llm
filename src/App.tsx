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
import "./App.css";

interface RecommendedSettings {
  memory_gb: number;
  recommended_model: string;
  recommended_ctx_size: number;
  recommended_gpu_layers: number;
}

function App() {
  const { theme, toggleTheme } = useTheme();
  const status = useServerStatus();
  const { logs, addLog } = useLogs();
  const isProduction = import.meta.env.PROD;

  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [baseModel, setBaseModel] = useState(""); // model or model_s
  const [isUncensored, setIsUncensored] = useState(false);
  const [port, setPort] = useState(() => {
    const saved = localStorage.getItem("port");
    return saved ? parseInt(saved) : 10345;
  });
  const [ctxSize, setCtxSize] = useState(() => {
    const saved = localStorage.getItem("ctxSize");
    return saved ? parseInt(saved) : 6000;
  });
  const [gpuLayers, setGpuLayers] = useState(() => {
    const saved = localStorage.getItem("gpuLayers");
    return saved ? parseInt(saved) : 41;
  });
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

  // Get recommended settings from backend
  useEffect(() => {
    const loadRecommendedSettings = async () => {
      try {
        const settings = await invoke<RecommendedSettings>("get_recommended_settings");
        // eslint-disable-next-line no-console
        console.log("Recommended settings:", settings);

        // IMPORTANT: Load uncensored preference FIRST (before setBaseModel)
        // This ensures currentModel is calculated correctly on the first render
        // and prevents duplicate toast notifications
        const savedUncensored = localStorage.getItem("isUncensored");
        const uncensored = savedUncensored === "true";
        if (savedUncensored !== null) {
          setIsUncensored(uncensored);
        }

        // Now set baseModel - currentModel will be calculated correctly
        setBaseModel(settings.recommended_model);
        addLog(
          `Auto-selected base model: ${settings.recommended_model} (RAM: ${settings.memory_gb} GB)`
        );

        // Set active model based on uncensored preference
        const initialModel = uncensored
          ? `${settings.recommended_model}_uncensored`
          : settings.recommended_model;

        try {
          // Check if the preferred model is downloaded
          const isDownloaded = await invoke<boolean>("check_model_downloaded", {
            modelName: initialModel,
          });

          if (isDownloaded) {
            // Set it as active if it exists
            await invoke<string>("set_active_model_command", { modelName: initialModel });
            addLog(`Active model set to: ${initialModel}`);
          } else {
            // If preferred model not downloaded, try base model
            const baseModelDownloaded = await invoke<boolean>("check_model_downloaded", {
              modelName: settings.recommended_model,
            });

            if (baseModelDownloaded) {
              await invoke<string>("set_active_model_command", {
                modelName: settings.recommended_model,
              });
              addLog(`Active model set to: ${settings.recommended_model}`);
            }
          }
        } catch (error) {
          console.error("Failed to set active model:", error);
        }

        // Set context size only if not manually set by user
        const savedCtxSize = localStorage.getItem("ctxSize");
        if (!savedCtxSize) {
          setCtxSize(settings.recommended_ctx_size);
          localStorage.setItem("ctxSize", settings.recommended_ctx_size.toString());
          addLog(
            `Auto-selected context size: ${settings.recommended_ctx_size} (RAM: ${settings.memory_gb} GB)`
          );
        }

        // Set GPU layers only if not manually set by user
        const savedGpuLayers = localStorage.getItem("gpuLayers");
        if (!savedGpuLayers) {
          setGpuLayers(settings.recommended_gpu_layers);
          localStorage.setItem("gpuLayers", settings.recommended_gpu_layers.toString());
        }
      } catch (error) {
        console.error("Failed to get recommended settings:", error);
        // Fallback to smaller model if detection fails
        setBaseModel("model_s");
        addLog("Failed to detect system settings, using fallback: model_s");

        // Set fallback context size if not set
        const savedCtxSize = localStorage.getItem("ctxSize");
        if (!savedCtxSize) {
          setCtxSize(6000);
          localStorage.setItem("ctxSize", "6000");
          addLog("Using fallback context size: 6k");
        }
      }
    };

    loadRecommendedSettings();
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
      const result = await invoke<string>("start_server", {
        port,
        ctxSize,
        gpuLayers,
      });
      toast.success(result);
      addLog(result);
    } catch (error) {
      toast.error(`Error: ${error}`);
      addLog(`Error: ${error}`);
    } finally {
      setIsBusy(false);
    }
  };

  const handlePortChange = (newPort: number) => {
    setPort(newPort);
    localStorage.setItem("port", newPort.toString());
  };

  const handleCtxSizeChange = (newCtxSize: number) => {
    setCtxSize(newCtxSize);
    localStorage.setItem("ctxSize", newCtxSize.toString());
  };

  const handleGpuLayersChange = (newGpuLayers: number) => {
    setGpuLayers(newGpuLayers);
    localStorage.setItem("gpuLayers", newGpuLayers.toString());
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
        port={port}
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
        onPortChange={handlePortChange}
        onCtxSizeChange={handleCtxSizeChange}
        onGpuLayersChange={handleGpuLayersChange}
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
