import { invoke } from "@tauri-apps/api/core";
import { useStore } from "./store";
import { toast } from "./utils";
import { relaunch } from "@tauri-apps/plugin-process";

export const startSeparation = async () => {
  const { uploadedFile, separationProgress, setCurrentStage } =
    useStore.getState();

  // check if file exists
  if (!uploadedFile) {
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
    await invoke("start_separation", { filePath: uploadedFile });
    setCurrentStage("separate");
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

export const abortSeparation = async () => {
  const { separationProgress, setSeparationProgress } = useStore.getState();
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
