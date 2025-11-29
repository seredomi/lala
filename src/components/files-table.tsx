import {
  Button,
  DataTable,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableHeader,
  TableRow,
  TableSelectAll,
  TableSelectRow,
  TableToolbar,
  TableToolbarContent,
  TableToolbarSearch,
  InlineLoading,
  IconButton,
} from "@carbon/react";
import { useFiles, StageInfo } from "../utils/use-files";
import {
  DocumentAdd,
  ArrowUpRight,
  Download,
  StopOutline,
  PortInput,
  CheckmarkFilled,
  TimeFilled,
  ErrorFilled,
} from "@carbon/icons-react";
import { FileWithStatus, TargetStage } from "../utils/schema";
import { createElement, useState } from "react";

// configuration records
const STATUS_CONFIG: Record<
  string,
  {
    text: string;
    color: string;
    icon: any;
  }
> = {
  empty: {
    text: "—",
    color: "var(--cds-text-secondary)",
    icon: null,
  },
  completed: {
    text: "done",
    color: "var(--cds-text-placeholder)",
    icon: CheckmarkFilled,
  },
  processing: {
    text: "processing",
    color: "var(--cds-support-info)",
    icon: InlineLoading,
  },
  queued: {
    text: "queued",
    color: "var(--cds-text-placeholder)",
    icon: TimeFilled,
  },
  failed: {
    text: "failed",
    color: "var(--cds-support-error)",
    icon: ErrorFilled,
  },
  cancelled: {
    text: "cancelled",
    color: "var(--cds-support-error)",
    icon: ErrorFilled,
  },
};

const ACTION_BUTTON_TEXT: Record<TargetStage, string> = {
  stems: "separate",
  midi: "transcribe",
  pdf: "convert",
};

interface StageCellProps {
  file: FileWithStatus;
  stage: TargetStage;
  stageInfo: StageInfo;
  isRowHovered: boolean;
  onProcess: (fileId: string, stage: TargetStage) => void;
  onCancel: (fileId: string) => void;
  onDownload: (fileId: string, assetType: string) => void;
}

const StageCell = ({
  file,
  stage,
  stageInfo,
  isRowHovered,
  onProcess,
  onCancel,
  onDownload,
}: StageCellProps) => {
  const cellStyle = {
    display: "flex",
    flexDirection: "column" as const,
    gap: "0.5rem",
    minHeight: "3rem",
    justifyContent: "center",
  };

  const statusRowStyle = {
    display: "flex",
    alignItems: "center",
    gap: "0.5rem",
  };

  const statusTextStyle = {
    fontSize: "0.875rem",
    color: STATUS_CONFIG[stageInfo.status].color,
  };

  // determine if this stage is currently processing
  const isThisStageProcessing =
    stageInfo.status === "processing" &&
    file.current_progress &&
    ((stage === "stems" && file.current_progress.asset_type === "original") ||
      file.current_progress.asset_type === stage);

  // check if this stage has failed
  // const thisStageError = file.assets.find(
  //   (a) =>
  //     (stage === "stems" && a.asset_type === "stem_piano") ||
  //     a.asset_type === stage,
  // )?.error_message;

  return (
    <div style={cellStyle}>
      {/* status row */}
      <div style={statusRowStyle}>
        <span
          style={{
            ...statusTextStyle,
            display: "flex",
            alignItems: "center",
            gap: "0.75rem",
          }}
        >
          {STATUS_CONFIG[stageInfo.status].icon && (
            <span style={{ display: "flex", alignItems: "center" }}>
              {createElement(STATUS_CONFIG[stageInfo.status].icon, {
                size: 16,
              })}
            </span>
          )}
          {isThisStageProcessing && file.current_progress && (
            <span
              style={{
                color: "var(--cds-support-info)",
              }}
            >
              {Math.round(file.current_progress.progress * 100)}%
            </span>
          )}
        </span>

        {/* action buttons */}
        {stageInfo.canDownload && (
          <IconButton
            size="sm"
            kind="tertiary"
            label={`download ${stage === "stems" ? "piano" : stage === "pdf" ? "sheet music" : stage}`}
            onClick={() =>
              onDownload(file.id, stage === "stems" ? "stem_piano" : stage)
            }
            style={{ visibility: isRowHovered ? "visible" : "hidden" }}
          >
            <Download />
          </IconButton>
        )}

        {stageInfo.canProcess && (
          <IconButton
            kind="tertiary"
            size="sm"
            label={ACTION_BUTTON_TEXT[stage]}
            onClick={() => onProcess(file.id, stage)}
            style={{
              visibility: isRowHovered ? "visible" : "hidden",
              marginLeft: "-15px",
            }}
          >
            <PortInput />
          </IconButton>
        )}

        {/* cancel button - only for the actively processing stage */}
        {isThisStageProcessing && (
          <IconButton
            // @ts-expect-error 'danger' works as a kind. unsure why cds doesnt enumerate it as an option
            kind="danger"
            size="sm"
            label="cancel"
            color="var(--cds-text-error)"
            onClick={() => onCancel(file.id)}
            style={{ visibility: isRowHovered ? "visible" : "hidden" }}
          >
            <StopOutline />
          </IconButton>
        )}
      </div>
    </div>
  );
};

interface FileRowProps {
  row: any;
  file: FileWithStatus;
  getRowProps: any;
  getSelectionProps: any;
  processToStage: (fileId: string, stage: TargetStage) => Promise<boolean>;
  cancelProcessing: (fileId: string) => Promise<boolean>;
  downloadAsset: (fileId: string, assetType: string) => Promise<boolean>;
  getStageInfo: (file: FileWithStatus, stage: TargetStage) => StageInfo;
}

const FileRow = ({
  row,
  file,
  getRowProps,
  getSelectionProps,
  processToStage,
  cancelProcessing,
  downloadAsset,
  getStageInfo,
}: FileRowProps) => {
  const [isRowHovered, setIsRowHovered] = useState(false);

  const stages: TargetStage[] = ["stems", "midi", "pdf"];
  const stageInfo = {
    stems: getStageInfo(file, "stems"),
    midi: getStageInfo(file, "midi"),
    pdf: getStageInfo(file, "pdf"),
  };

  return (
    <TableRow
      {...getRowProps({ row })}
      onMouseEnter={() => setIsRowHovered(true)}
      onMouseLeave={() => setIsRowHovered(false)}
    >
      <TableSelectRow {...getSelectionProps({ row })} />

      {/* initial audio */}
      <TableCell>
        <div
          style={{
            fontSize: "0.875rem",
            minHeight: "3rem",
            display: "flex",
            alignItems: "center",
          }}
        >
          {file.original_filename}
        </div>
      </TableCell>

      {/* stage columns */}
      {stages.map((stage) => (
        <TableCell key={stage}>
          <StageCell
            file={file}
            stage={stage}
            stageInfo={stageInfo[stage]}
            isRowHovered={isRowHovered}
            onProcess={processToStage}
            onCancel={cancelProcessing}
            onDownload={downloadAsset}
          />
        </TableCell>
      ))}
    </TableRow>
  );
};

export const FileTable = () => {
  const {
    files,
    uploadFile,
    processToStage,
    cancelProcessing,
    downloadAsset,
    getStageInfo,
  } = useFiles();

  const headers = [
    { header: "initial audio", key: "original_filename" },
    { header: "piano audio", key: "has_stems" },
    { header: "midi", key: "has_midi" },
    { header: "sheet music", key: "has_pdf" },
  ];

  const tableRows = files.map((file) => ({
    id: file.id,
    original_filename: file.original_filename,
    has_stems: "",
    has_midi: "",
    has_pdf: "",
  }));

  return (
    <DataTable headers={headers} rows={tableRows}>
      {({
        getHeaderProps,
        getRowProps,
        getSelectionProps,
        getTableProps,
        headers,
        rows,
      }) => (
        <TableContainer
          title="files"
          description="status of uploaded files"
          style={{ width: "100%" }}
        >
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
                <TableSelectAll {...getSelectionProps()} />
                {headers.map((header) => (
                  <TableHeader {...getHeaderProps({ header })}>
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
                    <FileRow
                      key={row.id}
                      row={row}
                      file={file}
                      getRowProps={getRowProps}
                      getSelectionProps={getSelectionProps}
                      processToStage={processToStage}
                      cancelProcessing={cancelProcessing}
                      downloadAsset={downloadAsset}
                      getStageInfo={getStageInfo}
                    />
                  );
                })
              ) : (
                <TableRow>
                  <TableCell colSpan={headers.length + 1}>
                    <div
                      style={{
                        display: "flex",
                        flexDirection: "column",
                        alignItems: "center",
                        justifyContent: "center",
                        width: "100%",
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
