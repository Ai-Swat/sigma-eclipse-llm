import { DownloadProgress } from "../types";

interface ProgressBarProps {
  downloadProgress: DownloadProgress | null;
}

export const ProgressBar = ({ downloadProgress }: ProgressBarProps) => {
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
    </div>
  );
};
