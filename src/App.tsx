import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Store } from "@tauri-apps/plugin-store";
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
  const [modelUrl, setModelUrl] = useState("https://example.com/model.zip");
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

  // Listen for download progress events
  useEffect(() => {
    const unlisten = listen<DownloadProgress>("download-progress", (event) => {
      const progress = event.payload;
      setDownloadProgress(progress);
      addLog(progress.message);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const addLog = (message: string) => {
    setLogs((prev) => [...prev, `[${new Date().toLocaleTimeString()}] ${message}`]);
  };

  const handleDownloadLlama = async () => {
    setIsDownloadingLlama(true);
    setDownloadProgress(null);
    addLog("Starting llama.cpp download...");
    try {
      const result = await invoke<string>("download_llama_cpp");
      addLog(result);
    } catch (error) {
      addLog(`Error: ${error}`);
    } finally {
      setIsDownloadingLlama(false);
      setDownloadProgress(null);
    }
  };

  const handleDownloadModel = async () => {
    if (!modelUrl.trim()) {
      addLog("Error: Please enter a model URL");
      return;
    }
    setIsDownloadingModel(true);
    setDownloadProgress(null);
    addLog(`Starting model download from ${modelUrl}...`);
    try {
      const result = await invoke<string>("download_model", { modelUrl });
      addLog(result);
    } catch (error) {
      addLog(`Error: ${error}`);
    } finally {
      setIsDownloadingModel(false);
      setDownloadProgress(null);
    }
  };

  const handleStartServer = async () => {
    addLog(`Starting server on port ${port}...`);
    try {
      const result = await invoke<string>("start_server", { port });
      addLog(result);
    } catch (error) {
      addLog(`Error: ${error}`);
    }
  };

  const handleStopServer = async () => {
    addLog("Stopping server...");
    try {
      const result = await invoke<string>("stop_server");
      addLog(result);
    } catch (error) {
      addLog(`Error: ${error}`);
    }
  };

  return (
    <main className="container">
      <div className="header-section">
        <h1><img src={logo} alt="Shield" className="logo-icon" /> Sigma Shield LLM</h1>
        <div className="theme-toggle-container">
          <button 
            className="settings-button" 
            onClick={() => setIsSettingsOpen(!isSettingsOpen)}
            title="Settings"
          >
            ⚙
          </button>
          <button className="theme-toggle" onClick={toggleTheme} title="Toggle theme">
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
                  <label>Model URL (zip with model.gguf and model.yaml):</label>
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

                <div className="button-group">
                  <button 
                    onClick={handleStartServer}
                    disabled={status.is_running}
                    className="start-button"
                  >
                    Start Server
                  </button>
                  <button 
                    onClick={handleStopServer}
                    disabled={!status.is_running}
                    className="stop-button"
                  >
                    Stop Server
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
          {status.is_running ? "Stop Server" : "Start Server"}
        </button>
      </div>

      <div className="section">
        <h2>Logs</h2>
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
