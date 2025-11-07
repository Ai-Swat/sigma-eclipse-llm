import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Store } from "@tauri-apps/plugin-store";
import { Toaster, toast } from "sonner";
import { BaseDirectory, exists } from "@tauri-apps/plugin-fs";
import logo from "./assets/logo2.png";
import "./App.css";

interface ServerStatus {
  is_running: boolean;
  message: string;
}

interface DownloadProgress {
  downloaded: number;
  total: number | null;
  percentage: number | null;
  message: string;
}

function App() {
  const [store, setStore] = useState<Store | null>(null);
  const [theme, setTheme] = useState<"dark" | "white">("dark");
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [modelUrl, setModelUrl] = useState("");
  const [port, setPort] = useState(10345);
  const [status, setStatus] = useState<ServerStatus>({
    is_running: false,
    message: "Not running",
  });
  const [logs, setLogs] = useState<string[]>([]);
  const [isDownloadingLlama, setIsDownloadingLlama] = useState(false);
  const [isDownloadingModel, setIsDownloadingModel] = useState(false);
  const [appDataPath, setAppDataPath] = useState("");
  const [downloadProgress, setDownloadProgress] = useState<DownloadProgress | null>(null);
  const [currentToastId, setCurrentToastId] = useState<string | number | null>(null);

  // Initialize store and load theme
  useEffect(() => {
    const initStore = async () => {
      try {
        const loadedStore = await Store.load("settings.json");
        setStore(loadedStore);
        
        const savedTheme = await loadedStore.get<string>("theme");
        if (savedTheme === "dark" || savedTheme === "white") {
          setTheme(savedTheme);
          document.documentElement.className = `theme-${savedTheme}`;
        } else {
          document.documentElement.className = "theme-dark";
        }
      } catch (error) {
        console.error("Failed to initialize store or load theme:", error);
        document.documentElement.className = "theme-dark";
      }
    };
    initStore();
  }, []);

  // Auto-detect and set model URL based on system memory
  useEffect(() => {
    
    const detectModelUrl = async () => {
      
      try {
        const memoryGb = await invoke<number>("get_system_memory_gb");
        console.log(`System memory detected: ${memoryGb} GB`);
        
        // If memory is less than 60GB, use the smaller model
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

  // Toggle theme and save to store
  const toggleTheme = async () => {
    const newTheme = theme === "dark" ? "white" : "dark";
    setTheme(newTheme);
    document.documentElement.className = `theme-${newTheme}`;
    
    if (store) {
      try {
        await store.set("theme", newTheme);
        await store.save();
      } catch (error) {
        console.error("Failed to save theme:", error);
      }
    }
  };

  // Check server status periodically
  useEffect(() => {
    const interval = setInterval(async () => {
      try {
        const status = await invoke<ServerStatus>("get_server_status");
        setStatus(status);
      } catch (error) {
        console.error("Failed to get status:", error);
      }
    }, 2000);

    return () => clearInterval(interval);
  }, []);

  // Get app data path on mount
  useEffect(() => {
    invoke<string>("get_app_data_path")
      .then((path) => setAppDataPath(path))
      .catch((error) => console.error("Failed to get app data path:", error));
  }, []);

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
        // Get the actual app data path from backend
        
        // Check if llama-server binary exists
        const llamaBinaryPath = `./bin/llama-server`;
        const llamaExists = await exists(llamaBinaryPath, { baseDir: BaseDirectory.AppData });
        
        // Check if model exists
        const modelPath = `./models/model.gguf`;
        const modelExists = await exists(modelPath, { baseDir: BaseDirectory.AppData });
        
        // Auto-download llama.cpp if missing
        if (!llamaExists && !isDownloadingLlama) {
          wasSomeDownloads = true;
          addLog("llama.cpp not found, downloading automatically...");
          setIsDownloadingLlama(true);
          setDownloadProgress(null);
          
          const toastId = toast.loading("Starting llama.cpp download...");
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

  // Listen for download progress events
  useEffect(() => {
    const unlisten = listen<DownloadProgress>("download-progress", (event) => {
      const progress = event.payload;
      setDownloadProgress(progress);
      addLog(progress.message);
      
      // Update toast with progress
      if (currentToastId) {
        const progressText = progress.percentage !== null 
          ? `${progress.percentage.toFixed(1)}%` 
          : `${(progress.downloaded / 1_048_576).toFixed(2)} MB`;
        
        toast.loading(`${progress.message} - ${progressText}`, {
          id: currentToastId,
        });
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [currentToastId]);

  const addLog = (message: string) => {
    setLogs((prev) => [...prev, `[${new Date().toLocaleTimeString()}] ${message}`]);
  };

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
      <div className="header-section">
        <h1><img src={logo} alt="Shield" className="logo-icon" /> Sigma Shield LLM</h1>
        <div className="theme-toggle-container">
          <button className="theme-toggle settings-button"
            onClick={() => setIsSettingsOpen(!isSettingsOpen)}
            title="Settings"
          >
            <span className="settings-button-icon">⚙</span>
          </button>
          <button className="theme-toggle theme-toggle-icon" onClick={toggleTheme} title="Toggle theme">
            {theme === "dark" ? "○" : "●"}
          </button>
        </div>
      </div>

      {/* Settings Panel Overlay */}
      {isSettingsOpen && (
        <div className="settings-overlay">
          <div className="settings-panel">
            <div className="settings-header">
              <h2>⚙️ Settings</h2>
              <button 
                className="close-button" 
                onClick={() => setIsSettingsOpen(false)}
                title="Close"
              >
                ✕
              </button>
            </div>
            
            <div className="settings-content">
              <div className="section">
                <h2>Setup</h2>
                <div className="form-group">
                  <label>App Data Directory:</label>
                  <input 
                    type="text" 
                    value={appDataPath} 
                    disabled 
                    className="readonly-input"
                  />
                </div>
                
                <div className="button-group">
                  <button 
                    onClick={handleDownloadLlama}
                    disabled={isDownloadingLlama}
                  >
                    {isDownloadingLlama ? "Downloading..." : "Download llama.cpp"}
                  </button>
                </div>

                {isDownloadingLlama && downloadProgress && (
                  <div className="progress-container">
                    <div className="progress-bar">
                      <div 
                        className="progress-fill" 
                        style={{ width: `${downloadProgress.percentage || 0}%` }}
                      ></div>
                    </div>
                    <div className="progress-text">
                      {downloadProgress.percentage !== null 
                        ? `${downloadProgress.percentage.toFixed(1)}%` 
                        : "Downloading..."}
                    </div>
                  </div>
                )}

                <div className="form-group">
                  <label>Model URL:</label>
                  <input
                    type="text"
                    value={modelUrl}
                    onChange={(e) => setModelUrl(e.target.value)}
                    placeholder="https://example.com/model.zip"
                  />
                </div>

                <div className="button-group">
                  <button 
                    onClick={handleDownloadModel}
                    disabled={isDownloadingModel || !modelUrl.trim()}
                  >
                    {isDownloadingModel ? "Downloading..." : "Download Model"}
                  </button>
                </div>

                {isDownloadingModel && downloadProgress && (
                  <div className="progress-container">
                    <div className="progress-bar">
                      <div 
                        className="progress-fill" 
                        style={{ width: `${downloadProgress.percentage || 0}%` }}
                      ></div>
                    </div>
                    <div className="progress-text">
                      {downloadProgress.percentage !== null 
                        ? `${downloadProgress.percentage.toFixed(1)}%` 
                        : "Downloading..."}
                    </div>
                  </div>
                )}
              </div>

              <div className="section">
                <h2>Server Configuration</h2>
                <div className="form-group">
                  <label>Port:</label>
                  <input
                    type="number"
                    value={port}
                    onChange={(e) => setPort(parseInt(e.target.value) || 10345)}
                    min="1024"
                    max="65535"
                  />
                </div>
              </div>

              <div className="section danger-section">
                <h2>Maintenance</h2>
                <p className="warning-text">Clear downloaded files to free up space</p>
                
                <div className="button-group">
              
                  <button 
                    onClick={handleClearAllData}
                    className="danger-button-severe"
                  >
                    Clear All Data
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}
      
      <div className="status-panel">
        <div className={`status-indicator ${status.is_running ? "running" : "stopped"}`}>
          <div className="status-dot"></div>
          <span>{status.is_running ? "Running" : "Stopped"}</span>
        </div>
        <p className="status-message">{status.message}</p>
        <button 
          className="server-toggle-button"
          onClick={status.is_running ? handleStopServer : handleStartServer}
        >
          {status.is_running ? "Stop" : "Start"}
        </button>
      </div>

      <div className="section">
        <h2 className="logs-header">Logs</h2>
        <div className="logs">
          {logs.map((log, index) => (
            <div key={index} className="log-entry">{log}</div>
          ))}
          {logs.length === 0 && (
            <div className="log-entry empty">No logs yet...</div>
          )}
        </div>
      </div>
    </main>
  );
}

export default App;
