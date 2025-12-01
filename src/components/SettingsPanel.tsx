import { useState, useEffect } from "react";
import { DownloadProgress, ServerStatus } from "../types";
import { ProgressBar } from "./ProgressBar";
import CloseIcon from "../icons/x-close.svg?react";
import "./SettingsPanel.css";

interface SettingsPanelProps {
  isOpen: boolean;
  appDataPath: string;
  baseModel: string;
  isUncensored: boolean;
  ctxSize: number;
  gpuLayers: number;
  isDownloadingLlama: boolean;
  isDownloadingModel: boolean;
  downloadProgress: DownloadProgress | null;
  status: ServerStatus;
  onClose: () => void;
  onDownloadLlama: () => void;
  onDownloadModel: () => void;
  onUncensoredChange: (checked: boolean) => void;
  onCtxSizeChange: (ctxSize: number) => void;
  onGpuLayersChange: (gpuLayers: number) => void;
  onRestoreDefaults: () => void;
  onClearAllData: () => void;
  isProduction: boolean;
}

export const SettingsPanel = ({
  isOpen,
  appDataPath,
  baseModel,
  isUncensored,
  ctxSize,
  gpuLayers,
  isDownloadingLlama,
  isDownloadingModel,
  downloadProgress,
  onClose,
  onDownloadLlama,
  onDownloadModel,
  onUncensoredChange,
  onCtxSizeChange,
  onGpuLayersChange,
  onRestoreDefaults,
  onClearAllData,
  isProduction,
}: SettingsPanelProps) => {
  // Local state for input values to allow empty strings during editing
  const [ctxSizeValue, setCtxSizeValue] = useState(ctxSize.toString());
  const [gpuLayersValue, setGpuLayersValue] = useState(gpuLayers.toString());

  // Sync local state with props when they change externally
  useEffect(() => {
    setCtxSizeValue(ctxSize.toString());
  }, [ctxSize]);

  useEffect(() => {
    setGpuLayersValue(gpuLayers.toString());
  }, [gpuLayers]);

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
              <label>Base Model:</label>
              <input
                type="text"
                value={baseModel}
                placeholder="model"
                disabled
                className="readonly-input"
              />
              <small className="help-text">Auto-selected based on system RAM</small>
            </div>

            <div className="form-group" style={{ marginTop: "1rem" }}>
              <label style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
                <input
                  type="checkbox"
                  checked={isUncensored}
                  onChange={(e) => onUncensoredChange(e.target.checked)}
                  disabled={isDownloadingModel}
                  style={{ width: "auto", cursor: "pointer" }}
                />
                <span style={{ fontWeight: "600" }}>Uncensored Model</span>
              </label>
              <small className="warning-text" style={{ display: "block", marginTop: "0.25rem" }}>
                ⚠️ Uncensored model may produce unfiltered content. Use with caution.
              </small>
              <small className="help-text" style={{ display: "block", marginTop: "0.25rem" }}>
                Will download and activate uncensored version if not already available
              </small>
            </div>

            <div className="button-group">
              <button
                className="primary-button"
                onClick={onDownloadModel}
                disabled={isDownloadingModel || !baseModel}
              >
                {isDownloadingModel ? "Downloading..." : "Download Current Model"}
              </button>
            </div>

            {isDownloadingModel && <ProgressBar downloadProgress={downloadProgress} />}
          </div>

          <div className="section">
            <h2>Server Configuration</h2>
            <div className="form-group">
              <label>Context Size:</label>
              <input
                type="number"
                value={ctxSizeValue}
                onChange={(e) => {
                  setCtxSizeValue(e.target.value);
                  const parsed = parseInt(e.target.value);
                  if (!isNaN(parsed)) {
                    onCtxSizeChange(parsed);
                  }
                }}
                onBlur={() => {
                  const value = parseInt(ctxSizeValue);
                  if (isNaN(value) || value < 6000) {
                    onCtxSizeChange(30000);
                    setCtxSizeValue('30000');
                  } else if (value > 100000) {
                    onCtxSizeChange(100000);
                    setCtxSizeValue('100000');
                  }
                }}
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
                value={gpuLayersValue}
                onChange={(e) => {
                  setGpuLayersValue(e.target.value);
                  const parsed = parseInt(e.target.value);
                  if (!isNaN(parsed)) {
                    onGpuLayersChange(parsed);
                  }
                }}
                onBlur={() => {
                  const value = parseInt(gpuLayersValue);
                  if (isNaN(value) || value < 0) {
                    onGpuLayersChange(0);
                    setGpuLayersValue('0');
                  } else if (value > 41) {
                    onGpuLayersChange(41);
                    setGpuLayersValue('41');
                  }
                }}
                min="0"
                max="41"
              />
              <small className="help-text">Range: 0 - 41 layers (0 = CPU only)</small>
            </div>

            <div className="button-group">
              <button className="secondary-button" onClick={onRestoreDefaults}>
                Restore Defaults
              </button>
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
