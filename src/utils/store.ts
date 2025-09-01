import { create } from "zustand";
import {
  AppConfig,
  AppConfigSchema,
  LoadingState,
  ErrorState,
  CurrentView,
  CurrentStage,
} from "./schema";
import { defaultAppConfig } from "./config";
import { toast } from "./utils";
import { FluidSelectSkeletonProps } from "@carbon/react/lib/components/FluidSelect";
import { sep } from "@tauri-apps/api/path";

interface AppStore {
  currentView: CurrentView;
  setCurrentView: (view: CurrentView) => void;

  currentStage: CurrentStage;
  setCurrentStage: (stage: CurrentStage) => void;

  appConfig: AppConfig;
  setAppConfig: (config: unknown) => void;

  uploadedFile: string | null;
  setUploadedFile: (val: string | null) => void;

  separationProgress: LoadingState | null;
  setSeparationProgress: (progress: LoadingState | null) => void;

  loadingState: LoadingState | null;
  setLoading: (loading: LoadingState | null) => void;
  clearLoading: () => void;

  errorState: ErrorState | null;
  setError: (error: ErrorState | null) => void;
  clearError: () => void;

  clearAll: () => void;
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

  uploadedFile: null,
  setUploadedFile: (val) =>
    set({ uploadedFile: val, separationProgress: null }),

  separationProgress: null,
  setSeparationProgress: (val) => set({ separationProgress: val }),

  loadingState: null,
  setLoading: (loading) => set({ loadingState: loading }),
  clearLoading: () => set({ loadingState: null }),

  errorState: null,
  setError: (error) => set({ errorState: error }),
  clearError: () => set({ errorState: null }),

  clearAll: () =>
    set({
      loadingState: null,
      errorState: null,
    }),
}));
