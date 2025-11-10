import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";
import { DownloadProgress } from "../types";

export const useDownloadProgress = (addLog: (message: string) => void) => {
  const [downloadProgress, setDownloadProgress] = useState<DownloadProgress | null>(null);
  const [currentToastId, setCurrentToastId] = useState<string | number | null>(null);

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
  }, [currentToastId, addLog]);

  return { downloadProgress, currentToastId, setCurrentToastId, setDownloadProgress };
};

