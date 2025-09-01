import { create } from "zustand";
import {
  AppConfig,
  AppConfigSchema,
  LoadingState,
  ErrorState,
  CurrentView,
} from "./schema";
import { defaultAppConfig } from "./config";
import { toast } from "./utils";

interface AppStore {
  currentView: CurrentView;
  setCurrentView: (view: CurrentView) => void;

  appConfig: AppConfig;
  setAppConfig: (config: unknown) => void;

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
