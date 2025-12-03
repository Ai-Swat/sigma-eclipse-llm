import "./UpdateDialog.css";

interface DownloadProgress {
  downloaded: number;
  total: number | null;
}

interface UpdateDialogProps {
  currentVersion: string;
  newVersion: string;
  isDownloading: boolean;
  downloadProgress: DownloadProgress | null;
  isInstalling: boolean;
  onUpdate: () => void;
  onDismiss: () => void;
}

export function UpdateDialog({
  newVersion,
  isDownloading,
  downloadProgress,
  isInstalling,
  onUpdate,
  onDismiss,
}: UpdateDialogProps) {
  const progressPercent = downloadProgress?.total
    ? Math.round((downloadProgress.downloaded / downloadProgress.total) * 100)
    : 0;

  const formatBytes = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  return (
    <div className="update-toast">
      <div className="update-toast-content">
        <div className="update-toast-icon">🚀</div>
        <div className="update-toast-text">
          <span className="update-toast-title">New version available</span>
        </div>
      </div>

      {isDownloading && downloadProgress && (
        <div className="update-toast-progress">
          <div className="update-progress-bar">
            <div
              className="update-progress-fill"
              style={{ width: `${progressPercent}%` }}
            />
          </div>
          <div className="update-progress-info">
            {downloadProgress.total ? (
              <span>
                {formatBytes(downloadProgress.downloaded)} / {formatBytes(downloadProgress.total)}
              </span>
            ) : (
              <span>{formatBytes(downloadProgress.downloaded)}</span>
            )}
          </div>
        </div>
      )}

      {isInstalling && (
        <div className="update-toast-installing">
          <span className="spinner"></span>
          <span>Installing...</span>
        </div>
      )}

      {!isDownloading && !isInstalling && (
        <div className="update-toast-actions">
          <button className="update-btn-dismiss" onClick={onDismiss}>
            Dismiss
          </button>
          <button className="update-btn-update" onClick={onUpdate}>
            Update
          </button>
        </div>
      )}
    </div>
  );
}
