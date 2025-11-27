import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";
import { DownloadProgress } from "../types";

export const useDownloadProgress = (addLog: (message: string) => void) => {
  const [downloadProgress, setDownloadProgress] = useState<DownloadProgress | null>(null);
  const [currentToastId, setCurrentToastId] = useState<string | number | null>(null);

  // Listen for download progress events - register ONCE
  useEffect(() => {
    const unlisten = listen<DownloadProgress>("download-progress", (event) => {
      const progress = event.payload;
      setDownloadProgress(progress);
      addLog(progress.message);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [addLog]);

  // Update toast when downloadProgress changes (separate from listener to avoid race conditions)
  useEffect(() => {
    if (downloadProgress && currentToastId) {
      const progressText =
        downloadProgress.percentage !== null
          ? `${downloadProgress.percentage.toFixed(1)}%`
          : `${(downloadProgress.downloaded / 1_048_576).toFixed(2)} MB`;

      toast.loading(`${downloadProgress.message} - ${progressText}`, {
        id: currentToastId,
      });
    }
  }, [downloadProgress, currentToastId]);

  return { downloadProgress, currentToastId, setCurrentToastId, setDownloadProgress };
};
