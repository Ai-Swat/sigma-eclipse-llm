import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Toaster, toast } from "sonner";
import { HeaderSection, SettingsPanel, StatusPanel, LogsSection } from "./components";
import { useTheme, useServerStatus, useLogs, useDownloadProgress, useAutoDownload } from "./hooks";
import "./App.css";

function App() {
  const { theme, toggleTheme } = useTheme();
  const status = useServerStatus();
  const { logs, addLog } = useLogs();
  
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [modelUrl, setModelUrl] = useState("");
  const [port, setPort] = useState(10345);
  const [appDataPath, setAppDataPath] = useState("");
  
  const { downloadProgress, setCurrentToastId, setDownloadProgress } = useDownloadProgress(addLog);
  
  const { isDownloadingLlama, isDownloadingModel, setIsDownloadingLlama, setIsDownloadingModel } = useAutoDownload({
    modelUrl,
    addLog,
    setCurrentToastId,
    setDownloadProgress,
  });

  // Auto-detect and set model URL based on system memory
  useEffect(() => {
    const detectModelUrl = async () => {
      try {
        const memoryGb = await invoke<number>("get_system_memory_gb");
        console.log(`System memory detected: ${memoryGb} GB`);
        
        // If memory is less than 16GB, use the smaller model
        if (memoryGb < 16) {
          setModelUrl("https://releases.sigmabrowser.com/dev/secure-llm/model_s.zip");
          addLog(`Auto-selected smaller model (RAM: ${memoryGb} GB < 16 GB)`);
        } else {
          setModelUrl("https://releases.sigmabrowser.com/dev/secure-llm/model.zip");
          addLog(`Auto-selected full model (RAM: ${memoryGb} GB >= 16 GB)`);
        }
      } catch (error) {
        console.error("Failed to detect system memory:", error);
        // Fallback to smaller model if detection fails
        setModelUrl("https://releases.sigmabrowser.com/dev/secure-llm/model_s.zip");
        addLog("Failed to detect RAM, using smaller model as fallback");
      }
    };
    
    detectModelUrl();
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Get app data path on mount
  useEffect(() => {
    invoke<string>("get_app_data_path")
      .then((path) => setAppDataPath(path))
      .catch((error) => console.error("Failed to get app data path:", error));
  }, []);

  const handleDownloadLlama = async () => {
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
    addLog(`Starting LLM on port ${port}...`);
    try {
      const result = await invoke<string>("start_server", { port });
      toast.success(result);
      addLog(result);
    } catch (error) {
      toast.error(`Error: ${error}`);
      addLog(`Error: ${error}`);
    }
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
        await new Promise(resolve => setTimeout(resolve, 500));
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
      <Toaster position="top-right" expand={true} richColors closeButton dir="ltr" />
      
      <HeaderSection 
        theme={theme}
        isSettingsOpen={isSettingsOpen}
        onToggleTheme={toggleTheme}
        onToggleSettings={() => setIsSettingsOpen(!isSettingsOpen)}
      />

      <SettingsPanel
        isOpen={isSettingsOpen}
        appDataPath={appDataPath}
        modelUrl={modelUrl}
        port={port}
        isDownloadingLlama={isDownloadingLlama}
        isDownloadingModel={isDownloadingModel}
        downloadProgress={downloadProgress}
        status={status}
        onClose={() => setIsSettingsOpen(false)}
        onDownloadLlama={handleDownloadLlama}
        onDownloadModel={handleDownloadModel}
        onModelUrlChange={setModelUrl}
        onPortChange={setPort}
        onClearAllData={handleClearAllData}
      />
      
      <StatusPanel 
        status={status}
        onStartServer={handleStartServer}
        onStopServer={handleStopServer}
      />

      <LogsSection logs={logs} />
    </main>
  );
}

export default App;
