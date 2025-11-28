import {
  Button,
  DataTable,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableExpandedRow,
  TableExpandHeader,
  TableExpandRow,
  TableHead,
  TableHeader,
  TableRow,
  TableSelectAll,
  TableSelectRow,
  TableToolbar,
  TableToolbarContent,
  TableToolbarSearch,
} from "@carbon/react";
import { useFiles } from "../utils/use-files";
import {
  DocumentAdd,
  ArrowUpRight,
  ArrowRight,
  Download,
  StopOutline,
  PortInput,
} from "@carbon/icons-react";
import { FileWithStatus, TargetStage } from "../utils/schema";
import { useEffect, useState } from "react";

// helper to determine stage status and next stage
const getStageInfo = (
  file: FileWithStatus,
  stage: "stems" | "midi" | "pdf",
) => {
  const stageAssetTypes = {
    stems: ["stem_piano", "stem_vocals", "stem_drums", "stem_bass"],
    midi: ["midi"],
    pdf: ["pdf"],
  };

  const relevantAssets = file.assets.filter((a) =>
    stageAssetTypes[stage].includes(a.asset_type),
  );

  const hasCompleted = relevantAssets.some((a) => a.status === "completed");
  const hasQueued = relevantAssets.some((a) => a.status === "queued");
  const hasProcessing = relevantAssets.some((a) => a.status === "processing");
  const hasFailed = relevantAssets.some((a) => a.status === "failed");
  const hasCancelled = relevantAssets.some((a) => a.status === "cancelled");

  let status: string = "—";
  let color: string = "var(--cds-text-secondary)";

  if (hasCompleted) {
    status = "complete";
    color = "var(--cds-text-success)";
  } else if (hasProcessing) {
    status = "processing";
    color = "var(--cds-text-primary)";
  } else if (hasQueued) {
    status = "queued";
    color = "var(--cds-text-secondary)";
  } else if (hasFailed) {
    status = "failed";
    color = "var(--cds-text-error)";
  } else if (hasCancelled) {
    status = "cancelled";
    color = "var(--cds-text-error)";
  }

  return {
    status,
    color,
    hasCompleted,
    hasQueued,
    hasProcessing,
    hasFailed,
    hasCancelled,
    isEmpty: !hasCompleted && !hasQueued && !hasProcessing,
    assets: relevantAssets.filter((a) => a.status === "completed"),
  };
};

// calculate how many stages away target is
const getStagesAway = (
  file: FileWithStatus,
  target: "stems" | "midi" | "pdf",
): number => {
  const stages = ["stems", "midi", "pdf"];
  const targetIndex = stages.indexOf(target);

  let currentStageIndex = -1;
  if (file.has_stems) currentStageIndex = 0;
  if (file.has_midi) currentStageIndex = 1;
  if (file.has_pdf) currentStageIndex = 2;

  return targetIndex - currentStageIndex;
};

// format file size
const formatFileSize = (bytes: number): string => {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
};

// format timestamp
const formatTimestamp = (timestamp: number): string => {
  return new Date(timestamp * 1000).toLocaleString();
};

// get file size if available
const getFileSize = (filePath: string): number | null => {
  // note: file size would need to be added to Asset schema and fetched from backend
  // for now, return null
  return null;
};

interface ProcessButtonProps {
  file: FileWithStatus;
  targetStage: TargetStage;
  stagesAway: number;
  onProcess: (fileId: string, stage: TargetStage) => void;
  size?: "sm" | "md" | "lg";
}

const ProcessButton = ({
  file,
  targetStage,
  stagesAway,
  onProcess,
  size = "sm",
}: ProcessButtonProps) => {
  const Icon = PortInput;

  return (
    <Button
      kind="ghost"
      size={size}
      renderIcon={Icon}
      onClick={() => onProcess(file.id, targetStage)}
    >
      process to {targetStage}
    </Button>
  );
};

interface CellContentProps {
  file: FileWithStatus;
  stage: "stems" | "midi" | "pdf";
  isExpanded: boolean;
  onProcess: (fileId: string, stage: TargetStage) => void;
  onCancel: (fileId: string) => void;
  onDownload: (fileId: string, assetType: string) => void;
}

const CellContent = ({
  file,
  stage,
  isExpanded,
  onProcess,
  onCancel,
  onDownload,
}: CellContentProps) => {
  const [isHovered, setIsHovered] = useState(false);
  const stageInfo = getStageInfo(file, stage);
  const stagesAway = getStagesAway(file, stage);
  const isProcessing = file.current_status === "processing";

  // map asset types to friendly names
  const stemNames: Record<string, string> = {
    stem_piano: "piano",
    stem_vocals: "vocals",
    stem_drums: "drums",
    stem_bass: "bass",
  };

  if (isExpanded) {
    // expanded view: show buttons/progress
    if (stageInfo.hasProcessing && file.current_asset_type?.includes(stage)) {
      // show progress and cancel button
      return (
        <div style={{ padding: "0.5rem 0" }}>
          <div style={{ marginBottom: "0.5rem" }}>
            <div style={{ fontWeight: 600 }}>
              {file.current_progress?.title || "processing..."}
            </div>
            <div
              style={{
                color: "var(--cds-text-secondary)",
                fontSize: "0.875rem",
              }}
            >
              {file.current_progress?.description || ""}
            </div>
          </div>
          <Button
            kind="danger--ghost"
            size="sm"
            renderIcon={StopOutline}
            onClick={() => onCancel(file.id)}
          >
            cancel
          </Button>
        </div>
      );
    } else if (stageInfo.hasCompleted) {
      // show download buttons
      return (
        <div
          style={{
            padding: "0.5rem 0",
            display: "flex",
            flexDirection: "column",
            gap: "0.5rem",
          }}
        >
          {stageInfo.assets.map((asset, idx) => {
            const fileName =
              asset.file_path.split("/").pop() || `track-${idx + 1}.wav`;
            const displayName = stemNames[asset.asset_type] || asset.asset_type;

            return (
              <Button
                key={asset.id}
                kind="ghost"
                size="sm"
                renderIcon={Download}
                onClick={() => onDownload(file.id, asset.asset_type)}
              >
                {displayName} ({fileName})
              </Button>
            );
          })}
        </div>
      );
    } else if (
      !isProcessing &&
      (stageInfo.isEmpty || stageInfo.hasFailed || stageInfo.hasCancelled)
    ) {
      // show process button
      return (
        <div style={{ padding: "0.5rem 0" }}>
          <ProcessButton
            file={file}
            targetStage={stage}
            stagesAway={stagesAway}
            onProcess={onProcess}
          />
        </div>
      );
    } else {
      return (
        <div style={{ padding: "0.5rem 0", color: stageInfo.color }}>ok</div>
      );
    }
  } else {
    // collapsed view: show status with hover button
    return (
      <div
        style={{ position: "relative" }}
        onMouseEnter={() => setIsHovered(true)}
        onMouseLeave={() => setIsHovered(false)}
      >
        <span style={{ color: stageInfo.color }}>{stageInfo.status}</span>

        {/* show hover button if not processing and stage is empty/cancelled/failed */}
        {isHovered &&
          !isProcessing &&
          (stageInfo.isEmpty ||
            stageInfo.hasFailed ||
            stageInfo.hasCancelled) && (
            <div style={{ position: "absolute", top: 0, left: 0, right: 0 }}>
              <ProcessButton
                file={file}
                targetStage={stage}
                stagesAway={stagesAway}
                onProcess={onProcess}
                size="sm"
              />
            </div>
          )}
      </div>
    );
  }
};

export const FileTable = () => {
  const {
    files,
    isLoading,
    uploadFile,
    processToStage,
    cancelProcessing,
    deleteFile,
    downloadAsset,
  } = useFiles();

  useEffect(() => {
    console.log("files", files);
  }, [files]);

  const headers = [
    {
      header: "initial audio",
      key: "original_filename",
    },
    {
      header: "instruments",
      key: "has_stems",
    },
    {
      header: "midi",
      key: "has_midi",
    },
    {
      header: "sheet music",
      key: "has_pdf",
    },
  ];

  return (
    <DataTable headers={headers} rows={files}>
      {({
        getCellProps,
        getHeaderProps,
        getRowProps,
        getSelectionProps,
        getTableProps,
        getExpandHeaderProps,
        getExpandedRowProps,
        headers,
        rows,
      }) => (
        <TableContainer title="files" description="status of uploaded files">
          <TableToolbar>
            <TableToolbarContent>
              <TableToolbarSearch
                onChange={(e) => console.log("search updated", e)}
                isExpanded={true}
              />
              <Button renderIcon={DocumentAdd} onClick={uploadFile}>
                add file
              </Button>
            </TableToolbarContent>
          </TableToolbar>
          <Table {...getTableProps()}>
            <TableHead>
              <TableRow>
                <TableExpandHeader
                  enableExpando={true}
                  {...getExpandHeaderProps()}
                />
                <TableSelectAll {...getSelectionProps()} />
                {headers.map((header) => (
                  <TableHeader key={header.key} {...getHeaderProps({ header })}>
                    {header.header}
                  </TableHeader>
                ))}
              </TableRow>
            </TableHead>
            <TableBody>
              {rows.length ? (
                rows.map((row) => {
                  const file = files.find((f) => f.id === row.id);
                  if (!file) return null;

                  return (
                    <>
                      <TableExpandRow key={row.id} {...getRowProps({ row })}>
                        <TableSelectRow {...getSelectionProps({ row })} />
                        {row.cells.map((cell) => {
                          // custom rendering for each cell
                          if (cell.info.header === "initial audio") {
                            return (
                              <TableCell
                                key={cell.id}
                                {...getCellProps({ cell })}
                              >
                                {cell.value}
                              </TableCell>
                            );
                          } else if (cell.info.header === "instruments") {
                            return (
                              <TableCell
                                key={cell.id}
                                {...getCellProps({ cell })}
                              >
                                <CellContent
                                  file={file}
                                  stage="stems"
                                  isExpanded={row.isExpanded}
                                  onProcess={processToStage}
                                  onCancel={cancelProcessing}
                                  onDownload={downloadAsset}
                                />
                              </TableCell>
                            );
                          } else if (cell.info.header === "midi") {
                            return (
                              <TableCell
                                key={cell.id}
                                {...getCellProps({ cell })}
                              >
                                <CellContent
                                  file={file}
                                  stage="midi"
                                  isExpanded={row.isExpanded}
                                  onProcess={processToStage}
                                  onCancel={cancelProcessing}
                                  onDownload={downloadAsset}
                                />
                              </TableCell>
                            );
                          } else if (cell.info.header === "sheet music") {
                            return (
                              <TableCell
                                key={cell.id}
                                {...getCellProps({ cell })}
                              >
                                <CellContent
                                  file={file}
                                  stage="pdf"
                                  isExpanded={Boolean(row.isExpanded)}
                                  onProcess={processToStage}
                                  onCancel={cancelProcessing}
                                  onDownload={downloadAsset}
                                />
                              </TableCell>
                            );
                          }
                          return (
                            <TableCell
                              key={cell.id}
                              {...getCellProps({ cell })}
                            >
                              {cell.value}
                            </TableCell>
                          );
                        })}
                      </TableExpandRow>
                      {row.isExpanded && (
                        <TableExpandedRow
                          {...getExpandedRowProps({ row })}
                          colSpan={headers.length + 2}
                        >
                          <div
                            style={{
                              display: "grid",
                              gridTemplateColumns: "1fr 1fr",
                              gridTemplateRows: "1fr 1fr",
                              padding: "0.5rem",
                              color: "var(--cds-text-secondary)",
                              fontSize: "0.875rem",
                            }}
                          >
                            <span style={{ fontStyle: "italic" }}>
                              uploaded:{" "}
                            </span>
                            <span style={{ fontFamily: "monospace" }}>
                              {formatTimestamp(file.created_at)}
                            </span>
                            <span style={{ fontStyle: "italic" }}>path: </span>
                            <span style={{ fontFamily: "monospace" }}>
                              {
                                file.assets.find(
                                  (a) => a.asset_type === "original",
                                )?.file_path
                              }
                            </span>

                            <span style={{ fontFamily: "monospace" }}></span>
                          </div>
                          {getFileSize(
                            file.assets.find((a) => a.asset_type === "original")
                              ?.file_path || "",
                          ) && (
                            <div>
                              size:{" "}
                              {formatFileSize(
                                getFileSize(
                                  file.assets.find(
                                    (a) => a.asset_type === "original",
                                  )?.file_path || "",
                                ) || 0,
                              )}
                            </div>
                          )}
                        </TableExpandedRow>
                      )}
                    </>
                  );
                })
              ) : (
                <TableRow>
                  <TableCell colSpan={headers.length + 2}>
                    <div
                      style={{
                        display: "flex",
                        flexDirection: "column",
                        alignItems: "center",
                        justifyContent: "center",
                        width: "100%",
                        padding: "2rem 0",
                        textWrap: "nowrap",
                      }}
                    >
                      <p>no audio files yet</p>
                      <div
                        style={{
                          display: "flex",
                          alignItems: "center",
                          gap: "0.5rem",
                        }}
                      >
                        <p>click add file to get started</p>
                        <ArrowUpRight />
                      </div>
                    </div>
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </TableContainer>
      )}
    </DataTable>
  );
};
