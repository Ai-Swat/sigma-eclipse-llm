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

function App() {
  const { theme, toggleTheme } = useTheme();
  const status = useServerStatus();
  const { logs, addLog } = useLogs();
  const isProduction = import.meta.env.PROD;

  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [modelUrl, setModelUrl] = useState("");
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

  const {
    isDownloadingLlama,
    isDownloadingModel,
    isModelAlreadyDownloaded,
    isLlamaAlreadyDownloaded,
    setIsDownloadingLlama,
    setIsDownloadingModel,
  } = useAutoDownload({
    modelUrl,
    addLog,
    setCurrentToastId,
    setDownloadProgress,
  });

  // Auto-detect and set model URL and context size based on system memory
  useEffect(() => {
    const detectSystemSettings = async () => {
      try {
        const memoryGb = await invoke<number>("get_system_memory_gb");
        // eslint-disable-next-line no-console
        console.log(`System memory detected: ${memoryGb} GB`);

        // Set model URL based on memory
        if (memoryGb < 16) {
          setModelUrl("https://releases.sigmabrowser.com/dev/secure-llm/model_s.zip");
          addLog(`Auto-selected smaller model (RAM: ${memoryGb} GB < 16 GB)`);
        } else {
          setModelUrl("https://releases.sigmabrowser.com/dev/secure-llm/model.zip");
          addLog(`Auto-selected full model (RAM: ${memoryGb} GB >= 16 GB)`);
        }

        // Set context size based on memory (only if not manually set by user)
        const savedCtxSize = localStorage.getItem("ctxSize");
        if (!savedCtxSize) {
          let autoCtxSize: number;
          if (memoryGb < 16) {
            autoCtxSize = 6000;
            addLog(`Auto-selected context size: 6k (RAM: ${memoryGb} GB < 16 GB)`);
          } else if (memoryGb >= 16 && memoryGb < 24) {
            autoCtxSize = 15000;
            addLog(`Auto-selected context size: 15k (RAM: ${memoryGb} GB between 16-24 GB)`);
          } else {
            autoCtxSize = 30000;
            addLog(`Auto-selected context size: 30k (RAM: ${memoryGb} GB >= 24 GB)`);
          }
          setCtxSize(autoCtxSize);
          localStorage.setItem("ctxSize", autoCtxSize.toString());
        }
      } catch (error) {
        console.error("Failed to detect system memory:", error);
        // Fallback to smaller model if detection fails
        setModelUrl("https://releases.sigmabrowser.com/dev/secure-llm/model_s.zip");
        addLog("Failed to detect RAM, using smaller model as fallback");

        // Set fallback context size if not set
        const savedCtxSize = localStorage.getItem("ctxSize");
        if (!savedCtxSize) {
          setCtxSize(6000);
          localStorage.setItem("ctxSize", "6000");
          addLog("Using fallback context size: 6k");
        }
      }
    };

    detectSystemSettings();
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

    if (!modelUrl.trim()) {
      toast.error("Please enter a model URL");
      addLog("Error: Please enter a model URL");
      return;
    }
    setIsDownloadingModel(true);
    setDownloadProgress(null);

    const toastId = toast.loading(`Starting model download...`);
    setCurrentToastId(toastId);
    addLog(`Starting model download from ${modelUrl}...`);

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
  };

  const handleStartServer = async () => {
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
    addLog("Stopping server...");
    try {
      const result = await invoke<string>("stop_server");
      toast.success(result);
      addLog(result);
    } catch (error) {
      toast.error(`Error: ${error}`);
      addLog(`Error: ${error}`);
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
        modelUrl={modelUrl}
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
        onModelUrlChange={setModelUrl}
        onPortChange={handlePortChange}
        onCtxSizeChange={handleCtxSizeChange}
        onGpuLayersChange={handleGpuLayersChange}
        onClearAllData={handleClearAllData}
        isProduction={isProduction}
      />

      <StatusPanel
        status={status}
        onStartServer={handleStartServer}
        onStopServer={handleStopServer}
      />

      {!isProduction && <LogsSection logs={logs} />}

      <div className="footer-section">
        <ThemeSwitcher theme={theme} onToggleTheme={toggleTheme} />
      </div>

      <Toaster position="bottom-right" expand={true} richColors closeButton dir="ltr" />
    </main>
  );
}

export default App;
