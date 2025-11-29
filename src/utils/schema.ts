import { z } from "zod";

export const CurrentViewSchema = z.enum(["main", "about", "error"]);

export const AppConfigSchema = z.object({
  file_upload: z.object({
    max_file_size_mb: z.number(),
    permitted_file_extensions: z.array(z.string()),
    max_upload_time_sec: z.number(),
  }),
});

export const ProcessingStatusSchema = z.enum([
  "queued",
  "processing",
  "completed",
  "failed",
  "cancelled",
]);

export const AssetTypeSchema = z.enum([
  "original",
  "stem_piano",
  "stem_vocals",
  "stem_drums",
  "stem_bass",
  "midi",
  "pdf",
]);

export const TargetStageSchema = z.enum(["stems", "midi", "pdf"]);

export const FileRecordSchema = z.object({
  id: z.string(),
  original_filename: z.string(),
  target_stage: TargetStageSchema.nullable(),
  created_at: z.number(),
});

export const AssetSchema = z.object({
  id: z.string(),
  file_id: z.string(),
  parent_asset_id: z.string().nullable(),
  asset_type: AssetTypeSchema,
  file_path: z.string(),
  status: ProcessingStatusSchema,
  error_message: z.string().nullable(),
  created_at: z.number(),
});

export const ProcessingProgressSchema = z.object({
  file_id: z.string(),
  asset_id: z.string(),
  asset_type: z.string(),
  title: z.string(),
  description: z.string(),
  progress: z.number().min(0).max(1),
});

// derived type for table display
export const FileWithStatusSchema = z.object({
  id: z.string(),
  original_filename: z.string(),
  created_at: z.number(),
  has_original: z.boolean(),
  has_stems: z.boolean(),
  has_midi: z.boolean(),
  has_pdf: z.boolean(),
  current_status: ProcessingStatusSchema.nullable(),
  current_asset_type: AssetTypeSchema.nullable(),
  current_progress: ProcessingProgressSchema.nullable(),
  error_message: z.string().nullable(),
  assets: z.array(AssetSchema),
  target_stage: TargetStageSchema.nullable(),
});

export type CurrentView = z.infer<typeof CurrentViewSchema>;
export type AppConfig = z.infer<typeof AppConfigSchema>;
export type ProcessingStatus = z.infer<typeof ProcessingStatusSchema>;
export type AssetType = z.infer<typeof AssetTypeSchema>;
export type FileRecord = z.infer<typeof FileRecordSchema>;
export type Asset = z.infer<typeof AssetSchema>;
export type ProcessingProgress = z.infer<typeof ProcessingProgressSchema>;
export type TargetStage = z.infer<typeof TargetStageSchema>;
export type FileWithStatus = z.infer<typeof FileWithStatusSchema>;
