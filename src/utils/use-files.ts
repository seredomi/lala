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

    console.log(
      "REFETCH completed, files:",
      filesWithStatus.map((f) => ({
        id: f.id.slice(0, 8),
        assets: f.assets.map((a) => ({ type: a.asset_type, status: a.status })),
        current_progress: f.current_progress,
      })),
    );

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

          console.log("PROGRESS EVENT:", {
            asset_type: progress.asset_type,
            title: progress.title,
            progress: progress.progress,
            timestamp: new Date().toISOString().slice(14, 23),
          });

          // update progress map
          progressMapRef.current.set(progress.file_id, progress);

          // if stage completed or failed, refetch to get updated asset statuses
          if (progress.title === "completed" || progress.title === "failed") {
            console.log("stage completed/failed, refetching files");
            // small delay to ensure backend has updated DB
            setTimeout(() => {
              loadFiles();
            }, 100);
          } else {
            // for ALL other progress updates (including "processing"), update in-place
            setFiles((prevFiles) =>
              prevFiles.map((file) =>
                file.id === progress.file_id
                  ? {
                      ...file,
                      current_progress: progress,
                      current_status: "processing",
                    }
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
      stems: ["stem_piano"],
      midi: ["midi"],
      pdf: ["pdf"],
    };

    const relevantAssets = file.assets.filter((a) =>
      stageAssetTypes[stage].includes(a.asset_type as AssetType),
    );

    const hasCompleted = relevantAssets.some((a) => a.status === "completed");
    const hasQueued = relevantAssets.some((a) => a.status === "queued");
    const hasProcessing =
      relevantAssets.some((a) => a.status === "processing") ||
      (relevantAssets.some((a) => a.status === "queued") &&
        file.current_progress &&
        file.current_progress.asset_type ===
          (stage === "stems" ? "original" : stage));
    const hasFailed = relevantAssets.some((a) => a.status === "failed");
    const hasCancelled = relevantAssets.some((a) => a.status === "cancelled");

    // special case: if original asset is processing, stems are being created
    const originalProcessing =
      stage === "stems" &&
      file.assets.some(
        (a) => a.asset_type === "original" && a.status === "processing",
      );

    // determine if this stage should show as "queued" based on target_stage
    const shouldShowQueued =
      file.target_stage &&
      !hasCompleted &&
      !hasProcessing &&
      !hasFailed &&
      !originalProcessing;
    const stageOrder = { stems: 0, midi: 1, pdf: 2 };
    const isInPath =
      file.target_stage &&
      stageOrder[stage] <=
        stageOrder[file.target_stage as keyof typeof stageOrder];

    let status: StageStatus = "empty";
    if (hasCompleted) status = "completed";
    else if (hasProcessing || originalProcessing) status = "processing";
    else if (hasQueued) status = "queued";
    else if (shouldShowQueued && isInPath) status = "queued";
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
        !file.target_stage &&
        (status === "empty" || status === "failed" || status === "cancelled"),
      canCancel:
        !!file.target_stage || status === "processing" || status === "queued",
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
