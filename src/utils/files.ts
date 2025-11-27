import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  Asset,
  FileRecord,
  FileWithStatus,
  ProcessingProgress,
  ProcessingStatus,
  TargetStage,
} from "./schema";
import { toast } from "./utils";

export const uploadFile = async (): Promise<string | null> => {
  try {
    // open file picker
    const selected = await open({
      title: "Select Audio File",
      multiple: false,
      filters: [
        {
          name: "Audio Files",
          extensions: ["wav", "mp3", "flac"],
        },
      ],
    });

    if (!selected) {
      return null; // user cancelled
    }

    const filePath = selected;
    const fileName = filePath.split("/").pop() || "audio.wav";

    // upload to backend
    const fileId: string = await invoke("upload_file", {
      sourcePath: filePath,
      originalFilename: fileName,
    });

    toast({
      kind: "success",
      title: "file uploaded",
      subtitle: fileName,
      actionButtonLabel: "ok",
      actionCloses: true,
    });

    return fileId;
  } catch (error) {
    console.error("failed to upload file:", error);
    toast({
      kind: "error",
      title: "upload failed",
      subtitle: "could not upload file",
      caption: String(error) || undefined,
      actionButtonLabel: "ok",
      actionCloses: true,
    });
    return null;
  }
};

export const listFiles = async (): Promise<FileRecord[]> => {
  try {
    const files: FileRecord[] = await invoke("list_files");
    return files;
  } catch (error) {
    console.error("failed to list files:", error);
    toast({
      kind: "error",
      title: "failed to load files",
      subtitle: String(error) || undefined,
      actionButtonLabel: "ok",
      actionCloses: true,
    });
    return [];
  }
};

export const listAssets = async (fileId: string): Promise<Asset[]> => {
  try {
    const assets: Asset[] = await invoke("list_assets", { fileId });
    return assets;
  } catch (error) {
    console.error("failed to list assets:", error);
    return [];
  }
};

export const getFilesWithStatus = async (
  progressMap: Map<string, ProcessingProgress>,
): Promise<FileWithStatus[]> => {
  const files = await listFiles();

  const filesWithStatus: FileWithStatus[] = await Promise.all(
    files.map(async (file) => {
      const assets = await listAssets(file.id);

      const hasOriginal = assets.some((a) => a.asset_type === "original");
      const hasStems = assets.some(
        (a) => a.asset_type === "stem_piano" && a.status === "completed",
      );
      const hasMidi = assets.some(
        (a) => a.asset_type === "midi" && a.status === "completed",
      );
      const hasPdf = assets.some(
        (a) => a.asset_type === "pdf" && a.status === "completed",
      );

      // find currently processing or queued asset
      const activeAsset = assets.find(
        (a) => a.status === "processing" || a.status === "queued",
      );

      // find any failed asset
      const failedAsset = assets.find((a) => a.status === "failed");

      let currentStatus: ProcessingStatus | null = null;
      let currentAssetType = null;
      let currentProgress = null;
      let errorMessage = null;

      if (activeAsset) {
        currentStatus = activeAsset.status;
        currentAssetType = activeAsset.asset_type;
        // get progress from map if processing
        if (activeAsset.status === "processing") {
          currentProgress = progressMap.get(file.id) || null;
        }
      } else if (failedAsset) {
        currentStatus = "failed";
        currentAssetType = failedAsset.asset_type;
        errorMessage = failedAsset.error_message;
      }

      return {
        ...file,
        has_original: hasOriginal,
        has_stems: hasStems,
        has_midi: hasMidi,
        has_pdf: hasPdf,
        current_status: currentStatus,
        current_asset_type: currentAssetType,
        current_progress: currentProgress,
        error_message: errorMessage,
        assets,
      };
    }),
  );

  return filesWithStatus;
};

export const processToStage = async (
  fileId: string,
  targetStage: TargetStage,
): Promise<boolean> => {
  try {
    await invoke("process_to_stage", { fileId, targetStage });
    toast({
      kind: "success",
      title: "processing started",
      subtitle: `processing to ${targetStage}`,
      actionButtonLabel: "ok",
      actionCloses: true,
    });
    return true;
  } catch (error) {
    console.error("failed to start processing:", error);
    toast({
      kind: "error",
      title: "processing failed",
      subtitle: String(error) || undefined,
      actionButtonLabel: "ok",
      actionCloses: true,
    });
    return false;
  }
};

export const cancelProcessing = async (fileId: string): Promise<boolean> => {
  try {
    await invoke("cancel_processing", { fileId });
    toast({
      kind: "success",
      title: "processing cancelled",
      subtitle: "stopped all queued jobs",
      actionButtonLabel: "ok",
      actionCloses: true,
    });
    return true;
  } catch (error) {
    console.error("failed to cancel processing:", error);
    toast({
      kind: "error",
      title: "cancel failed",
      subtitle: String(error) || undefined,
      actionButtonLabel: "ok",
      actionCloses: true,
    });
    return false;
  }
};

export const downloadAsset = async (
  asset: Asset,
  defaultFileName: string,
): Promise<boolean> => {
  try {
    const extension = asset.file_path.split(".").pop() || "wav";

    const outputPath = await save({
      title: `Save ${asset.asset_type}`,
      defaultPath: `${defaultFileName}-${asset.asset_type}.${extension}`,
      filters: [
        {
          name: `${extension.toUpperCase()} File`,
          extensions: [extension],
        },
      ],
    });

    if (!outputPath) {
      return false; // user cancelled
    }

    await invoke("download_asset", {
      assetPath: asset.file_path,
      destination: outputPath,
    });

    toast({
      kind: "success",
      title: "download complete",
      subtitle: `saved to ${outputPath}`,
      actionButtonLabel: "ok",
      actionCloses: true,
    });

    return true;
  } catch (error) {
    console.error("failed to download asset:", error);
    toast({
      kind: "error",
      title: "download failed",
      subtitle: String(error) || undefined,
      actionButtonLabel: "ok",
      actionCloses: true,
    });
    return false;
  }
};

export const deleteFile = async (fileId: string): Promise<boolean> => {
  try {
    await invoke("delete_file", { fileId });
    toast({
      kind: "success",
      title: "file deleted",
      subtitle: "removed file and all processed assets",
      actionButtonLabel: "ok",
      actionCloses: true,
    });
    return true;
  } catch (error) {
    console.error("failed to delete file:", error);
    toast({
      kind: "error",
      title: "delete failed",
      subtitle: String(error) || undefined,
      actionButtonLabel: "ok",
      actionCloses: true,
    });
    return false;
  }
};
