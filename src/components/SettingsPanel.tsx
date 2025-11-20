import { DownloadProgress, ServerStatus } from "../types";
import { ProgressBar } from "./ProgressBar";
import CloseIcon from "../icons/x-close.svg?react";
import "./SettingsPanel.css";

interface SettingsPanelProps {
  isOpen: boolean;
  appDataPath: string;
  recommendedModel: string;
  port: number;
  ctxSize: number;
  gpuLayers: number;
  isDownloadingLlama: boolean;
  isDownloadingModel: boolean;
  downloadProgress: DownloadProgress | null;
  status: ServerStatus;
  onClose: () => void;
  onDownloadLlama: () => void;
  onDownloadModel: () => void;
  onPortChange: (port: number) => void;
  onCtxSizeChange: (ctxSize: number) => void;
  onGpuLayersChange: (gpuLayers: number) => void;
  onClearAllData: () => void;
  isProduction: boolean;
}

export const SettingsPanel = ({
  isOpen,
  appDataPath,
  recommendedModel,
  port,
  ctxSize,
  gpuLayers,
  isDownloadingLlama,
  isDownloadingModel,
  downloadProgress,
  onClose,
  onDownloadLlama,
  onDownloadModel,
  onPortChange,
  onCtxSizeChange,
  onGpuLayersChange,
  onClearAllData,
  isProduction,
}: SettingsPanelProps) => {
  if (!isOpen) return null;

  return (
    <div className="settings-overlay">
      <div className="settings-panel">
        <div className="header-section">
          <h2>Settings</h2>
          <button className="transparent-hover-button close-button" onClick={onClose} title="Close">
            <CloseIcon width={20} height={20} />
          </button>
        </div>

        <div className="settings-content">
          <div className="section">
            <h2>Setup</h2>
            {!isProduction && (
              <>
                <div className="form-group">
                  <label>App Data Directory:</label>
                  <input type="text" value={appDataPath} disabled className="readonly-input" />
                </div>

                <div className="button-group">
                  <button
                    className="primary-button"
                    onClick={onDownloadLlama}
                    disabled={isDownloadingLlama}
                  >
                    {isDownloadingLlama ? "Downloading..." : "Download llama.cpp"}
                  </button>
                </div>

                {isDownloadingLlama && <ProgressBar downloadProgress={downloadProgress} />}
              </>
            )}

            <div className="form-group">
              <label>Model Name:</label>
              <input
                type="text"
                value={recommendedModel}
                placeholder="model"
                disabled
                className="readonly-input"
              />
              <small className="help-text">Auto-selected based on system RAM</small>
            </div>

            <div className="button-group">
              <button
                className="primary-button"
                onClick={onDownloadModel}
                disabled={isDownloadingModel || !recommendedModel}
              >
                {isDownloadingModel ? "Downloading..." : "Download Model"}
              </button>
            </div>

            {isDownloadingModel && <ProgressBar downloadProgress={downloadProgress} />}
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

            <div className="form-group">
              <label>Context Size:</label>
              <input
                type="number"
                value={ctxSize}
                onChange={(e) => onCtxSizeChange(parseInt(e.target.value) || 30000)}
                min="6000"
                max="100000"
                step="1000"
              />
              <small className="help-text">Range: 6,000 - 100,000 tokens</small>
            </div>

            <div className="form-group">
              <label>GPU Layers:</label>
              <input
                type="number"
                value={gpuLayers}
                onChange={(e) => onGpuLayersChange(parseInt(e.target.value) || 41)}
                min="0"
                max="41"
              />
              <small className="help-text">Range: 0 - 41 layers (0 = CPU only)</small>
            </div>
          </div>

          <div className="section danger-section">
            <h2>Maintenance</h2>
            <p className="warning-text">Clear downloaded files to free up space</p>

            <div className="button-group">
              <button onClick={onClearAllData} className="danger-button-severe">
                Clear All Data
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
