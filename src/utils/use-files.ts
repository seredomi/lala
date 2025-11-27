import { useEffect, useRef, useState } from "react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { FileWithStatus, ProcessingProgress, TargetStage } from "./schema";
import {
  getFilesWithStatus,
  uploadFile as uploadFileApi,
  processToStage as processToStageApi,
  cancelProcessing as cancelProcessingApi,
  deleteFile as deleteFileApi,
  downloadAsset as downloadAssetApi,
} from "./files";

export const useFiles = () => {
  const [files, setFiles] = useState<FileWithStatus[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const progressMapRef = useRef<Map<string, ProcessingProgress>>(new Map());

  // load files from backend
  const loadFiles = async () => {
    setIsLoading(true);
    const filesWithStatus = await getFilesWithStatus(progressMapRef.current);
    setFiles(filesWithStatus);
    setIsLoading(false);
  };

  // initial load
  useEffect(() => {
    loadFiles();
  }, []);

  // listen to processing progress events
  useEffect(() => {
    let unlisten: UnlistenFn | null = null;

    const setupListener = async () => {
      unlisten = await listen<ProcessingProgress>(
        "processing_progress",
        (event) => {
          const progress = event.payload;

          // update progress map
          progressMapRef.current.set(progress.file_id, progress);

          // if stage completed or failed, refetch to get updated asset statuses
          if (progress.title === "completed" || progress.title === "failed") {
            // small delay to ensure backend has updated DB
            setTimeout(() => {
              loadFiles();
            }, 100);
          } else {
            // just update progress in-place without refetching
            setFiles((prevFiles) =>
              prevFiles.map((file) =>
                file.id === progress.file_id
                  ? { ...file, current_progress: progress }
                  : file,
              ),
            );
          }
        },
      );
    };

    setupListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  // wrapper functions that auto-refresh
  const uploadFile = async () => {
    const fileId = await uploadFileApi();
    if (fileId) {
      await loadFiles();
    }
    return fileId;
  };

  const processToStage = async (fileId: string, targetStage: TargetStage) => {
    const success = await processToStageApi(fileId, targetStage);
    if (success) {
      await loadFiles();
    }
    return success;
  };

  const cancelProcessing = async (fileId: string) => {
    const success = await cancelProcessingApi(fileId);
    if (success) {
      // clear progress for this file
      progressMapRef.current.delete(fileId);
      await loadFiles();
    }
    return success;
  };

  const deleteFile = async (fileId: string) => {
    const success = await deleteFileApi(fileId);
    if (success) {
      progressMapRef.current.delete(fileId);
      await loadFiles();
    }
    return success;
  };

  const downloadAsset = async (fileId: string, assetType: string) => {
    const file = files.find((f) => f.id === fileId);
    const asset = file?.assets.find((a) => a.asset_type === assetType);
    if (asset && file) {
      return await downloadAssetApi(
        asset,
        file.original_filename.split(".")[0],
      );
    }
    return false;
  };

  return {
    files,
    isLoading,
    uploadFile,
    processToStage,
    cancelProcessing,
    deleteFile,
    downloadAsset,
    refresh: loadFiles,
  };
};
