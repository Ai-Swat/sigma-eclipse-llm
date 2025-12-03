import { useState, useEffect, useCallback, useRef } from "react";
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
  const updateRef = useRef<Update | null>(null);

  // Listen for update-available event from Rust backend and fetch Update object
  useEffect(() => {
    const unlisten = listen<UpdateInfo>("update-available", async (event) => {
      console.log("[Updater] Received update-available event:", event.payload);
      setUpdateInfo({
        currentVersion: event.payload.currentVersion,
        newVersion: event.payload.newVersion,
        body: event.payload.body,
      });
      setUpdateAvailable(true);
      
      // Fetch the actual Update object for download capability
      try {
        console.log("[Updater] Fetching Update object via check()...");
        const result = await check();
        console.log("[Updater] check() result:", result);
        if (result) {
          updateRef.current = result;
          console.log("[Updater] Update object stored in ref");
        }
      } catch (error) {
        console.error("[Updater] Failed to fetch update object:", error);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Manual check for updates
  const checkForUpdates = useCallback(async () => {
    try {
      console.log("[Updater] Manual check for updates...");
      const result = await check();
      console.log("[Updater] Manual check result:", result);
      if (result) {
        updateRef.current = result;
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
      console.error("[Updater] Failed to check for updates:", error);
      return false;
    }
  }, []);

  // Download and install update
  const downloadAndInstall = useCallback(async () => {
    console.log("[Updater] downloadAndInstall called");
    console.log("[Updater] updateRef.current:", updateRef.current);
    
    let currentUpdate = updateRef.current;
    
    // If no update object, try to check again
    if (!currentUpdate) {
      console.log("[Updater] No update object, fetching via check()...");
      try {
        const result = await check();
        console.log("[Updater] check() result:", result);
        if (!result) {
          console.error("[Updater] No update available from check()");
          return;
        }
        currentUpdate = result;
        updateRef.current = result;
      } catch (error) {
        console.error("[Updater] Failed to check for updates:", error);
        return;
      }
    }

    try {
      console.log("[Updater] Starting download and install...");
      setIsDownloading(true);
      setDownloadProgress({ downloaded: 0, total: null });

      let downloaded = 0;
      let contentLength: number | null = null;

      await currentUpdate.downloadAndInstall((event) => {
        console.log("[Updater] Download event:", event);
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

      console.log("[Updater] Download complete, relaunching...");
      // Relaunch the app after installation
      await relaunch();
    } catch (error) {
      console.error("[Updater] Failed to download/install update:", error);
      setIsDownloading(false);
      setIsInstalling(false);
    }
  }, []);

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
