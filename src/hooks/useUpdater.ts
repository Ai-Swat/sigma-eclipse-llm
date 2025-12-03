import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { check, Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

interface UpdateInfo {
  currentVersion: string;
  newVersion: string;
  body?: string;
}

interface DownloadProgress {
  downloaded: number;
  total: number | null;
}

export function useUpdater() {
  const [updateAvailable, setUpdateAvailable] = useState(false);
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [isDownloading, setIsDownloading] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState<DownloadProgress | null>(null);
  const [isInstalling, setIsInstalling] = useState(false);
  const [update, setUpdate] = useState<Update | null>(null);

  // Listen for update-available event from Rust backend
  useEffect(() => {
    const unlisten = listen<UpdateInfo>("update-available", (event) => {
      setUpdateInfo({
        currentVersion: event.payload.currentVersion,
        newVersion: event.payload.newVersion,
        body: event.payload.body,
      });
      setUpdateAvailable(true);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Manual check for updates
  const checkForUpdates = useCallback(async () => {
    try {
      const result = await check();
      if (result) {
        setUpdate(result);
        setUpdateInfo({
          currentVersion: result.currentVersion,
          newVersion: result.version,
          body: result.body ?? undefined,
        });
        setUpdateAvailable(true);
        return true;
      }
      return false;
    } catch (error) {
      console.error("Failed to check for updates:", error);
      return false;
    }
  }, []);

  // Download and install update
  const downloadAndInstall = useCallback(async () => {
    if (!update) {
      // If no update object, try to check again
      const result = await check();
      if (!result) return;
      setUpdate(result);
    }

    const currentUpdate = update || (await check());
    if (!currentUpdate) return;

    try {
      setIsDownloading(true);
      setDownloadProgress({ downloaded: 0, total: null });

      let downloaded = 0;
      let contentLength: number | null = null;

      await currentUpdate.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            contentLength = event.data.contentLength ?? null;
            setDownloadProgress({ downloaded: 0, total: contentLength });
            break;
          case "Progress":
            downloaded += event.data.chunkLength;
            setDownloadProgress({ downloaded, total: contentLength });
            break;
          case "Finished":
            setIsDownloading(false);
            setIsInstalling(true);
            break;
        }
      });

      // Relaunch the app after installation
      await relaunch();
    } catch (error) {
      console.error("Failed to download/install update:", error);
      setIsDownloading(false);
      setIsInstalling(false);
    }
  }, [update]);

  // Dismiss update notification
  const dismissUpdate = useCallback(() => {
    setUpdateAvailable(false);
  }, []);

  return {
    updateAvailable,
    updateInfo,
    isDownloading,
    downloadProgress,
    isInstalling,
    checkForUpdates,
    downloadAndInstall,
    dismissUpdate,
  };
}

