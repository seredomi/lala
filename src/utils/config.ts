import { AppConfig } from "./schema";

export const defaultAppConfig: AppConfig = {
  file_upload: {
    max_file_size_mb: 500,
    permitted_file_extensions: ["wav"],
    max_upload_time_sec: 300,
  },
};
