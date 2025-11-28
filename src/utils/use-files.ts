import { useEffect, useRef, useState } from "react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import {
  FileWithStatus,
  ProcessingProgress,
  TargetStage,
  AssetType,
} from "./schema";
import {
  getFilesWithStatus,
  uploadFile as uploadFileApi,
  processToStage as processToStageApi,
  cancelProcessing as cancelProcessingApi,
  deleteFile as deleteFileApi,
  downloadAsset as downloadAssetApi,
} from "./files";

// helper types for stage status
export type StageStatus =
  | "completed"
  | "processing"
  | "queued"
  | "failed"
  | "cancelled"
  | "empty";

export interface StageInfo {
  status: StageStatus;
  assets: Array<{ id: string; asset_type: string; file_path: string }>;
  canProcess: boolean;
  canCancel: boolean;
  canDownload: boolean;
}

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

  // helper to get stage info for a file
  const getStageInfo = (
    file: FileWithStatus,
    stage: "stems" | "midi" | "pdf",
  ): StageInfo => {
    const stageAssetTypes: Record<string, AssetType[]> = {
      stems: ["stem_piano", "stem_vocals", "stem_drums", "stem_bass"],
      midi: ["midi"],
      pdf: ["pdf"],
    };

    const relevantAssets = file.assets.filter((a) =>
      stageAssetTypes[stage].includes(a.asset_type as AssetType),
    );

    const hasCompleted = relevantAssets.some((a) => a.status === "completed");
    const hasQueued = relevantAssets.some((a) => a.status === "queued");
    const hasProcessing = relevantAssets.some((a) => a.status === "processing");
    const hasFailed = relevantAssets.some((a) => a.status === "failed");
    const hasCancelled = relevantAssets.some((a) => a.status === "cancelled");

    let status: StageStatus = "empty";
    if (hasCompleted) status = "completed";
    else if (hasProcessing) status = "processing";
    else if (hasQueued) status = "queued";
    else if (hasFailed) status = "failed";
    else if (hasCancelled) status = "cancelled";

    const isProcessingAnything = file.current_status === "processing";

    return {
      status,
      assets: relevantAssets
        .filter((a) => a.status === "completed")
        .map((a) => ({
          id: a.id,
          asset_type: a.asset_type,
          file_path: a.file_path,
        })),
      canProcess:
        !isProcessingAnything &&
        (status === "empty" || status === "failed" || status === "cancelled"),
      canCancel: status === "processing" || status === "queued",
      canDownload: status === "completed",
    };
  };

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
    getStageInfo, // expose helper
  };
};
