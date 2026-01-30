import { DownloadProgress } from "../types";
import "./ProgressBar.css";

interface ProgressBarProps {
  downloadProgress: DownloadProgress | null;
  onCancel: () => void;
}

export const ProgressBar = ({ downloadProgress, onCancel }: ProgressBarProps & { onCancel: () => void }) => {
  if (!downloadProgress) return null;

  return (
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
      <div>
        <button
          className="cancel-button"
          onClick={onCancel}
        >
          Cancel
        </button>
      </div>
    </div>
  );
};
