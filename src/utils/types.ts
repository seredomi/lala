export type AppConfig = {
  file_upload: {
    max_file_size_mb: number;
    permitted_file_types: string[];
    max_upload_time_sec: number;
  };
};

export type LoadingState = {
  title: string;
  description: string;
  progress?: number; // 0-100
};

export type ErrorState = {
  title: string;
  description: string;
};

export type InfoState = ErrorState;
