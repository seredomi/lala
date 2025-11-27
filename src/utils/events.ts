import { listen } from "@tauri-apps/api/event";
import { ProcessingProgress } from "./schema";

export const listenToProcessingProgress = (
  callback: (progress: ProcessingProgress) => void,
) => {
  return listen<ProcessingProgress>("processing_progress", (event) => {
    callback(event.payload);
  });
};
