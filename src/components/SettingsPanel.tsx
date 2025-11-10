import { DownloadProgress, ServerStatus } from "../types";
import { ProgressBar } from "./ProgressBar";

interface SettingsPanelProps {
  isOpen: boolean;
  appDataPath: string;
  modelUrl: string;
  port: number;
  isDownloadingLlama: boolean;
  isDownloadingModel: boolean;
  downloadProgress: DownloadProgress | null;
  status: ServerStatus;
  onClose: () => void;
  onDownloadLlama: () => void;
  onDownloadModel: () => void;
  onModelUrlChange: (url: string) => void;
  onPortChange: (port: number) => void;
  onClearAllData: () => void;
}

export const SettingsPanel = ({
  isOpen,
  appDataPath,
  modelUrl,
  port,
  isDownloadingLlama,
  isDownloadingModel,
  downloadProgress,
  status,
  onClose,
  onDownloadLlama,
  onDownloadModel,
  onModelUrlChange,
  onPortChange,
  onClearAllData,
}: SettingsPanelProps) => {
  if (!isOpen) return null;

  return (
    <div className="settings-overlay">
      <div className="settings-panel">
        <div className="settings-header">
          <h2>⚙️ Settings</h2>
          <button 
            className="close-button" 
            onClick={onClose}
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
                onClick={onDownloadLlama}
                disabled={isDownloadingLlama}
              >
                {isDownloadingLlama ? "Downloading..." : "Download llama.cpp"}
              </button>
            </div>

            {isDownloadingLlama && (
              <ProgressBar downloadProgress={downloadProgress} />
            )}

            <div className="form-group">
              <label>Model URL:</label>
              <input
                type="text"
                value={modelUrl}
                onChange={(e) => onModelUrlChange(e.target.value)}
                placeholder="https://example.com/model.zip"
              />
            </div>

            <div className="button-group">
              <button 
                onClick={onDownloadModel}
                disabled={isDownloadingModel || !modelUrl.trim()}
              >
                {isDownloadingModel ? "Downloading..." : "Download Model"}
              </button>
            </div>

            {isDownloadingModel && (
              <ProgressBar downloadProgress={downloadProgress} />
            )}
          </div>

          <div className="section">
            <h2>Server Configuration</h2>
            <div className="form-group">
              <label>Port:</label>
              <input
                type="number"
                value={port}
                onChange={(e) => onPortChange(parseInt(e.target.value) || 10345)}
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
                onClick={onClearAllData}
                className="danger-button-severe"
              >
                Clear All Data
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

