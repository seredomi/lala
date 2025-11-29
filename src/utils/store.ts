import { create } from "zustand";
import { AppConfig, AppConfigSchema, FileWithStatus } from "./schema";
import { defaultAppConfig } from "./config";
import { toast } from "./utils";

interface AppStore {
  currentView: "main" | "about" | "error";
  setCurrentView: (view: "main" | "about" | "error") => void;

  appConfig: AppConfig;
  setAppConfig: (config: unknown) => void;

  files: FileWithStatus[];
  setFiles: (files: FileWithStatus[]) => void;

  selectedFileId: string | null;
  setSelectedFileId: (id: string | null) => void;
}

export const useStore = create<AppStore>((set) => ({
  currentView: "main",
  setCurrentView: (view) => set({ currentView: view }),

  appConfig: defaultAppConfig,
  setAppConfig: (unknownConfig) => {
    const result = AppConfigSchema.safeParse(unknownConfig);
    if (result.success) {
      set({ appConfig: result.data });
    } else {
      console.warn(
        "invalid backend config, keeping frontend defaults",
        result.error,
      );
      toast({
        kind: "info-square",
        title: "invalid backend config",
        subtitle: "keeping clientside config",
        actionButtonLabel: "ok",
        actionCloses: true,
      });
    }
  },

  files: [],
  setFiles: (files) => set({ files }),

  selectedFileId: null,
  setSelectedFileId: (id) => set({ selectedFileId: id }),
}));
