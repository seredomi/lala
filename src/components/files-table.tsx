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
} from "@carbon/react";
import { useFiles, StageInfo } from "../utils/use-files";
import {
  DocumentAdd,
  ArrowUpRight,
  Download,
  StopOutline,
  PortInput,
} from "@carbon/icons-react";
import { FileWithStatus, TargetStage } from "../utils/schema";
import { useState } from "react";

// configuration records
const STATUS_COLORS: Record<string, string> = {
  completed: "var(--cds-support-success)",
  processing: "var(--cds-support-info)",
  queued: "var(--cds-text-secondary)",
  failed: "var(--cds-support-error)",
  cancelled: "var(--cds-support-error)",
  empty: "var(--cds-text-secondary)",
};

const STATUS_TEXT: Record<string, string> = {
  empty: "—",
  completed: "completed",
  processing: "processing",
  queued: "queued",
  failed: "failed",
  cancelled: "cancelled",
};

const ACTION_BUTTON_TEXT: Record<TargetStage, string> = {
  stems: "separate",
  midi: "transcribe",
  pdf: "convert",
};

const STEM_NAMES: Record<string, string> = {
  stem_piano: "piano",
  stem_vocals: "vocals",
  stem_drums: "drums",
  stem_bass: "bass",
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
    alignItems: "center",
    gap: "0.5rem",
    minHeight: "3rem",
  };

  const statusTextStyle = {
    fontSize: "0.875rem",
    color: STATUS_COLORS[stageInfo.status],
  };

  // processing state
  if (stageInfo.status === "processing") {
    return (
      <div style={cellStyle}>
        <div style={{ flex: 1 }}>
          <div style={{ ...statusTextStyle, fontWeight: 600 }}>
            {file.current_progress?.title || "processing"}
          </div>
          <div
            style={{
              color: "var(--cds-text-secondary)",
              fontSize: "0.75rem",
            }}
          >
            {file.current_progress?.description || ""}
          </div>
        </div>
        <Button
          kind="danger--ghost"
          size="sm"
          hasIconOnly
          iconDescription="cancel"
          renderIcon={StopOutline}
          onClick={() => onCancel(file.id)}
          style={{ visibility: isRowHovered ? "visible" : "hidden" }}
        />
      </div>
    );
  }

  // completed state with downloads
  if (stageInfo.canDownload) {
    return (
      <div style={cellStyle}>
        <span style={statusTextStyle}>{STATUS_TEXT[stageInfo.status]}</span>
        <div style={{ display: "flex", gap: "0.25rem" }}>
          {stage === "stems" ? (
            stageInfo.assets.map((asset) => (
              <Button
                key={asset.id}
                kind="ghost"
                size="sm"
                hasIconOnly
                iconDescription={`download ${STEM_NAMES[asset.asset_type] || asset.asset_type}`}
                renderIcon={Download}
                onClick={() => onDownload(file.id, asset.asset_type)}
                style={{ visibility: isRowHovered ? "visible" : "hidden" }}
              />
            ))
          ) : (
            <Button
              kind="ghost"
              size="sm"
              hasIconOnly
              iconDescription={`download ${stage}`}
              renderIcon={Download}
              onClick={() => onDownload(file.id, stage)}
              style={{ visibility: isRowHovered ? "visible" : "hidden" }}
            />
          )}
        </div>
      </div>
    );
  }

  // queued/failed/cancelled/empty state
  return (
    <div style={cellStyle}>
      <span style={statusTextStyle}>{STATUS_TEXT[stageInfo.status]}</span>
      {stageInfo.canProcess && (
        <Button
          kind="ghost"
          size="sm"
          hasIconOnly
          iconDescription={ACTION_BUTTON_TEXT[stage]}
          renderIcon={PortInput}
          onClick={() => onProcess(file.id, stage)}
          style={{ visibility: isRowHovered ? "visible" : "hidden" }}
        />
      )}
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
    { header: "instruments", key: "has_stems" },
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
          <Table {...getTableProps()} style={{ width: "100%" }}>
            <TableHead>
              <TableRow>
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
                        padding: "2rem 0",
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
