import "./UpdateDialog.css";

interface DownloadProgress {
  downloaded: number;
  total: number | null;
}

interface UpdateDialogProps {
  currentVersion: string;
  newVersion: string;
  releaseNotes?: string;
  isDownloading: boolean;
  downloadProgress: DownloadProgress | null;
  isInstalling: boolean;
  onUpdate: () => void;
  onDismiss: () => void;
}

export function UpdateDialog({
  currentVersion,
  newVersion,
  releaseNotes,
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
    <div className="update-dialog-overlay">
      <div className="update-dialog">
        <div className="update-dialog-header">
          <div className="update-icon">🚀</div>
          <h2>Доступно обновление!</h2>
        </div>

        <div className="update-dialog-content">
          <div className="version-info">
            <span className="version-current">{currentVersion}</span>
            <span className="version-arrow">→</span>
            <span className="version-new">{newVersion}</span>
          </div>

          {releaseNotes && (
            <div className="release-notes">
              <h4>Что нового:</h4>
              <p>{releaseNotes}</p>
            </div>
          )}

          {isDownloading && downloadProgress && (
            <div className="download-progress">
              <div className="update-progress-bar">
                <div
                  className="update-progress-fill"
                  style={{ width: `${progressPercent}%` }}
                />
              </div>
              <div className="download-info">
                {downloadProgress.total ? (
                  <span>
                    {formatBytes(downloadProgress.downloaded)} / {formatBytes(downloadProgress.total)} ({progressPercent}%)
                  </span>
                ) : (
                  <span>{formatBytes(downloadProgress.downloaded)}</span>
                )}
              </div>
            </div>
          )}

          {isInstalling && (
            <div className="installing-status">
              <span className="spinner"></span>
              <span>Установка обновления...</span>
            </div>
          )}
        </div>

        <div className="update-dialog-actions">
          {!isDownloading && !isInstalling && (
            <>
              <button className="update-button-secondary" onClick={onDismiss}>
                Позже
              </button>
              <button className="update-button-primary" onClick={onUpdate}>
                Обновить сейчас
              </button>
            </>
          )}
          {isDownloading && (
            <span className="download-status">Загрузка обновления...</span>
          )}
        </div>
      </div>
    </div>
  );
}

