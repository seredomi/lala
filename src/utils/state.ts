import { atom } from "recoil";
import { AppConfig, ErrorState, LoadingState } from "./types";

export const loadingState = atom<LoadingState | null>({
  key: "loading",
  default: null,
});

export const errorState = atom<ErrorState | null>({
  key: "error",
  default: null,
});

export const appConfigState = atom<AppConfig>({
  key: "appConfig",
  default: {
    file_upload: {
      max_file_size_mb: 500,
      permitted_file_types: ["flac"],
      max_upload_time_sec: 300,
    },
  },
});
