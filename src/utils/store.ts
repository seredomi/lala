import { create } from "zustand";
import {
  AppConfig,
  AppConfigSchema,
  LoadingState,
  CurrentView,
  CurrentStage,
} from "./schema";
import { defaultAppConfig } from "./config";
import { toast } from "./utils";

interface AppStore {
  currentView: CurrentView;
  setCurrentView: (view: CurrentView) => void;

  currentStage: CurrentStage;
  setCurrentStage: (stage: CurrentStage) => void;

  appConfig: AppConfig;
  setAppConfig: (config: unknown) => void;

  selectedFilepath: string | null;
  setSelectedFilepath: (val: string | null) => void;

  availableStems: string[];
  setAvailableStems: (stems: string[]) => void;

  separationProgress: LoadingState | null;
  setSeparationProgress: (progress: LoadingState | null) => void;

  downloadProgress: LoadingState | null;
  setDownloadProgress: (progress: LoadingState | null) => void;
}

export const useStore = create<AppStore>((set) => ({
  currentView: "main",
  setCurrentView: (view) => set({ currentView: view }),

  currentStage: "upload",
  setCurrentStage: (stage) => set({ currentStage: stage }),

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

  selectedFilepath: null,
  setSelectedFilepath: (val) =>
    set({
      selectedFilepath: val,
      separationProgress: null,
      downloadProgress: null,
      availableStems: [],
    }),

  availableStems: [],
  setAvailableStems: (stems) => set({ availableStems: stems }),

  separationProgress: null,
  setSeparationProgress: (val) => set({ separationProgress: val }),

  downloadProgress: null,
  setDownloadProgress: (val) => set({ downloadProgress: val }),
}));
