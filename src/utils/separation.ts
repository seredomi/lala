import { invoke } from "@tauri-apps/api/core";
import { useStore } from "./store";
import { toast } from "./utils";
import { relaunch } from "@tauri-apps/plugin-process";
import { save } from "@tauri-apps/plugin-dialog";

export const startSeparation = async () => {
  const {
    selectedFilepath,
    separationProgress,
    setCurrentStage,
    setAvailableStems,
  } = useStore.getState();

  // check if file exists
  if (!selectedFilepath) {
    toast({
      kind: "warning",
      title: "no file selected",
      subtitle: "please select a file first",
      actionButtonLabel: "ok",
      actionCloses: true,
    });
    return false;
  }

  // check if separation already in progress
  if (separationProgress) {
    setCurrentStage("separate");
    return true;
  }

  try {
    setCurrentStage("separate");
    const stems: string[] = await invoke("start_separation", {
      filePath: selectedFilepath,
    });
    setAvailableStems(stems);
    return true;
  } catch (error) {
    console.error("failed to separate:", error);
    console.error(String(error));
    toast({
      kind: "error",
      title: "failed to separate ",
      subtitle: "try again, or try restarting the app",
      caption: String(error) || undefined,
      actionButtonLabel: "restart app",
      onActionButtonClick: () => relaunch(),
    });
    return false;
  }
};

export const downloadStem = async (stemName: string) => {
  const { selectedFilepath } = useStore.getState();

  if (!selectedFilepath) {
    toast({
      kind: "error",
      title: "no file uploaded",
      subtitle: "please upload and separate a file first",
      actionButtonLabel: "ok",
      actionCloses: true,
    });
    return false;
  }

  try {
    // get original filename without extension
    const originalName =
      selectedFilepath.split("/").pop()?.split(".")[0] || "audio";

    const outputPath = await save({
      title: `Save ${stemName}`,
      defaultPath: `${originalName}-${stemName}.wav`,
      filters: [
        {
          name: "WAV Audio",
          extensions: ["wav"],
        },
      ],
    });

    if (!outputPath) {
      return false; // user cancelled
    }

    await invoke("download_stem", {
      stemName,
      outputPath,
    });

    return true;
  } catch (error) {
    console.error("failed to download stem:", error);
    toast({
      kind: "error",
      title: "download failed",
      subtitle: "failed to save the file",
      caption: String(error) || undefined,
      actionButtonLabel: "ok",
      actionCloses: true,
    });
    return false;
  }
};

export const abortSeparation = async () => {
  const { setSeparationProgress } = useStore.getState();
  try {
    await invoke("abort_separation");
    return true;
  } catch (error) {
    if (String(error) === "no separation to cancel")
      setSeparationProgress({
        title: "cancelled",
        description: "separation successfully cancelled",
        progress: 0,
      });
    console.error("failed to cancel:", error);
    toast({
      kind: "error",
      title: "error",
      subtitle: "failed to stop separation",
      caption: String(error) || undefined,
      actionButtonLabel: "ok",
      actionCloses: true,
    });
    return false;
  }
};
