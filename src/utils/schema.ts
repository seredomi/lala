import { z } from "zod";

export const CurrentViewSchema = z.enum(["main", "about", "error"]);
export const CurrentStageSchema = z.enum(["upload", "separate", "transcribe"]);

export const AppConfigSchema = z.object({
  file_upload: z.object({
    max_file_size_mb: z.number(),
    permitted_file_extensions: z.array(z.string()),
    max_upload_time_sec: z.number(),
  }),
});

export const LoadingStateSchema = z.object({
  title: z.string(),
  description: z.string(),
  progress: z.number().min(0).max(100).optional(),
});

export const ErrorStateSchema = z.object({
  title: z.string(),
  description: z.string(),
  data: z.any().optional(),
});

export type CurrentView = z.infer<typeof CurrentViewSchema>;
export type CurrentStage = z.infer<typeof CurrentStageSchema>;
export type AppConfig = z.infer<typeof AppConfigSchema>;
export type LoadingState = z.infer<typeof LoadingStateSchema>;
export type ErrorState = z.infer<typeof ErrorStateSchema>;
