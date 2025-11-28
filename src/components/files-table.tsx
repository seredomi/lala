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

// format timestamp
const formatTimestamp = (timestamp: number): string => {
  return new Date(timestamp * 1000).toLocaleString();
};

// status colors
const getStatusColor = (status: string): string => {
  switch (status) {
    case "completed":
      return "var(--cds-support-success)";
    case "processing":
      return "var(--cds-support-info)";
    case "queued":
      return "var(--cds-text-secondary)";
    case "failed":
    case "cancelled":
      return "var(--cds-support-error)";
    default:
      return "var(--cds-text-secondary)";
  }
};

// status display text
const getStatusText = (status: string): string => {
  return status === "empty" ? "—" : status;
};

// get action button text
const getActionButtonText = (stage: "stems" | "midi" | "pdf"): string => {
  switch (stage) {
    case "stems":
      return "separate instruments";
    case "midi":
      return "transcribe to midi";
    case "pdf":
      return "convert to sheet music";
  }
};

interface CellContentProps {
  file: FileWithStatus;
  stage: "stems" | "midi" | "pdf";
  stageInfo: StageInfo;
  isExpanded: boolean;
  isRowHovered: boolean;
  onProcess: (fileId: string, stage: TargetStage) => void;
  onCancel: (fileId: string) => void;
  onDownload: (fileId: string, assetType: string) => void;
}

const CellContent = ({
  file,
  stage,
  stageInfo,
  isExpanded,
  isRowHovered,
  onProcess,
  onCancel,
  onDownload,
}: CellContentProps) => {
  if (isExpanded) {
    // expanded view: just show status text
    return (
      <div style={{ color: getStatusColor(stageInfo.status) }}>
        {getStatusText(stageInfo.status)}
      </div>
    );
  } else {
    // collapsed view: show status or icon buttons on hover
    return (
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: "0.5rem",
          minHeight: "2rem",
        }}
      >
        {!isRowHovered ? (
          <span style={{ color: getStatusColor(stageInfo.status) }}>
            {getStatusText(stageInfo.status)}
          </span>
        ) : (
          <>
            {stageInfo.canProcess && (
              <Button
                kind="ghost"
                size="sm"
                hasIconOnly
                iconDescription={getActionButtonText(stage)}
                renderIcon={PortInput}
                onClick={() => onProcess(file.id, stage)}
              />
            )}

            {stageInfo.canCancel && (
              <Button
                kind="danger--ghost"
                size="sm"
                hasIconOnly
                iconDescription="cancel"
                renderIcon={StopOutline}
                onClick={() => onCancel(file.id)}
              />
            )}

            {stageInfo.canDownload && (
              <Button
                kind="ghost"
                size="sm"
                hasIconOnly
                iconDescription={
                  stage === "stems"
                    ? "download piano stem"
                    : `download ${stage}`
                }
                renderIcon={Download}
                onClick={() =>
                  onDownload(file.id, stage === "stems" ? "stem_piano" : stage)
                }
              />
            )}
          </>
        )}
      </div>
    );
  }
};

interface ExpandedCellContentProps {
  file: FileWithStatus;
  stage: "stems" | "midi" | "pdf";
  stageInfo: StageInfo;
  onProcess: (fileId: string, stage: TargetStage) => void;
  onCancel: (fileId: string) => void;
  onDownload: (fileId: string, assetType: string) => void;
}

const ExpandedCellContent = ({
  file,
  stage,
  stageInfo,
  onProcess,
  onCancel,
  onDownload,
}: ExpandedCellContentProps) => {
  // map asset types to friendly names
  const stemNames: Record<string, string> = {
    stem_piano: "piano",
    stem_vocals: "vocals",
    stem_drums: "drums",
    stem_bass: "bass",
  };

  const stageName = stage === "stems" ? "instruments" : stage;

  if (stageInfo.status === "processing" && file.current_progress) {
    return (
      <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem" }}>
        <div>
          <div style={{ fontWeight: 600 }}>{file.current_progress.title}</div>
          <div
            style={{
              color: "var(--cds-text-secondary)",
              fontSize: "0.875rem",
            }}
          >
            {file.current_progress.description}
          </div>
        </div>
        <Button
          kind="danger--ghost"
          size="sm"
          renderIcon={StopOutline}
          onClick={() => onCancel(file.id)}
        >
          cancel {stageName}
        </Button>
      </div>
    );
  } else if (stageInfo.canDownload) {
    return (
      <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem" }}>
        {stage === "stems" ? (
          <>
            {/* individual stem downloads */}
            {stageInfo.assets.map((asset) => {
              const displayName =
                stemNames[asset.asset_type] || asset.asset_type;
              return (
                <Button
                  key={asset.id}
                  kind="ghost"
                  size="sm"
                  renderIcon={Download}
                  onClick={() => onDownload(file.id, asset.asset_type)}
                >
                  download {displayName}
                </Button>
              );
            })}
            {/* download all button */}
            {stageInfo.assets.length > 1 && (
              <Button
                kind="primary"
                size="sm"
                renderIcon={Download}
                onClick={() => {
                  // download all stems
                  stageInfo.assets.forEach((asset) => {
                    onDownload(file.id, asset.asset_type);
                  });
                }}
              >
                download all stems
              </Button>
            )}
          </>
        ) : (
          <Button
            kind="ghost"
            size="sm"
            renderIcon={Download}
            onClick={() => onDownload(file.id, stage)}
          >
            download {stageName}
          </Button>
        )}
      </div>
    );
  } else if (stageInfo.canProcess) {
    return (
      <Button
        kind="ghost"
        size="sm"
        renderIcon={PortInput}
        onClick={() => onProcess(file.id, stage)}
      >
        {getActionButtonText(stage)}
      </Button>
    );
  } else {
    return null;
  }
};

// separate component for each row to manage its own hover state
interface FileRowProps {
  row: any;
  file: FileWithStatus;
  getRowProps: any;
  getSelectionProps: any;
  getExpandedRowProps: any;
  headers: any[];
  processToStage: (fileId: string, stage: TargetStage) => Promise<boolean>;
  cancelProcessing: (fileId: string) => Promise<boolean>;
  downloadAsset: (fileId: string, assetType: string) => Promise<boolean>;
  getStageInfo: (
    file: FileWithStatus,
    stage: "stems" | "midi" | "pdf",
  ) => StageInfo;
}

const FileRow = ({
  row,
  file,
  getRowProps,
  getSelectionProps,
  getExpandedRowProps,
  headers,
  processToStage,
  cancelProcessing,
  downloadAsset,
  getStageInfo,
}: FileRowProps) => {
  const [isRowHovered, setIsRowHovered] = useState(false);

  const stemsInfo = getStageInfo(file, "stems");
  const midiInfo = getStageInfo(file, "midi");
  const pdfInfo = getStageInfo(file, "pdf");

  return (
    <>
      <TableExpandRow
        {...getRowProps({ row })}
        onMouseEnter={() => setIsRowHovered(true)}
        onMouseLeave={() => setIsRowHovered(false)}
      >
        <TableSelectRow {...getSelectionProps({ row })} />

        {/* initial audio */}
        <TableCell>{file.original_filename}</TableCell>

        {/* instruments */}
        <TableCell>
          <CellContent
            file={file}
            stage="stems"
            stageInfo={stemsInfo}
            isExpanded={row.isExpanded}
            isRowHovered={isRowHovered}
            onProcess={processToStage}
            onCancel={cancelProcessing}
            onDownload={downloadAsset}
          />
        </TableCell>

        {/* midi */}
        <TableCell>
          <CellContent
            file={file}
            stage="midi"
            stageInfo={midiInfo}
            isExpanded={row.isExpanded}
            isRowHovered={isRowHovered}
            onProcess={processToStage}
            onCancel={cancelProcessing}
            onDownload={downloadAsset}
          />
        </TableCell>

        {/* sheet music */}
        <TableCell>
          <CellContent
            file={file}
            stage="pdf"
            stageInfo={pdfInfo}
            isExpanded={row.isExpanded}
            isRowHovered={isRowHovered}
            onProcess={processToStage}
            onCancel={cancelProcessing}
            onDownload={downloadAsset}
          />
        </TableCell>
      </TableExpandRow>

      {row.isExpanded && (
        <TableExpandedRow
          {...getExpandedRowProps({ row })}
          colSpan={headers.length + 2}
        >
          <table style={{ width: "100%", tableLayout: "fixed" }}>
            <tbody>
              <tr>
                {/* spacer for expand icon */}
                <td style={{ width: "48px" }}></td>
                {/* spacer for checkbox */}
                <td style={{ width: "48px" }}></td>

                {/* initial audio column */}
                <td style={{ padding: "1rem", verticalAlign: "top" }}>
                  <div
                    style={{
                      display: "flex",
                      flexDirection: "column",
                      gap: "0.25rem",
                      color: "var(--cds-text-secondary)",
                      fontSize: "0.75rem",
                    }}
                  >
                    <div>
                      <strong>uploaded:</strong>{" "}
                      {formatTimestamp(file.created_at)}
                    </div>
                    <div
                      style={{
                        fontFamily: "monospace",
                        wordBreak: "break-all",
                      }}
                    >
                      <strong>path:</strong>{" "}
                      {
                        file.assets.find((a) => a.asset_type === "original")
                          ?.file_path
                      }
                    </div>
                    {file.error_message && (
                      <div style={{ color: "var(--cds-support-error)" }}>
                        <strong>error:</strong> {file.error_message}
                      </div>
                    )}
                  </div>
                </td>

                {/* instruments column */}
                <td style={{ padding: "1rem", verticalAlign: "top" }}>
                  <ExpandedCellContent
                    file={file}
                    stage="stems"
                    stageInfo={stemsInfo}
                    onProcess={processToStage}
                    onCancel={cancelProcessing}
                    onDownload={downloadAsset}
                  />
                </td>

                {/* midi column */}
                <td style={{ padding: "1rem", verticalAlign: "top" }}>
                  <ExpandedCellContent
                    file={file}
                    stage="midi"
                    stageInfo={midiInfo}
                    onProcess={processToStage}
                    onCancel={cancelProcessing}
                    onDownload={downloadAsset}
                  />
                </td>

                {/* sheet music column */}
                <td style={{ padding: "1rem", verticalAlign: "top" }}>
                  <ExpandedCellContent
                    file={file}
                    stage="pdf"
                    stageInfo={pdfInfo}
                    onProcess={processToStage}
                    onCancel={cancelProcessing}
                    onDownload={downloadAsset}
                  />
                </td>
              </tr>
            </tbody>
          </table>
        </TableExpandedRow>
      )}
    </>
  );
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
    getStageInfo,
  } = useFiles();

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

  // transform files for DataTable - provide simple cell values
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
        getExpandHeaderProps,
        getExpandedRowProps,
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
          <Table
            {...getTableProps()}
            style={{ width: "100%", tableLayout: "fixed" }}
          >
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
                    <FileRow
                      key={row.id}
                      row={row}
                      file={file}
                      getRowProps={getRowProps}
                      getSelectionProps={getSelectionProps}
                      getExpandedRowProps={getExpandedRowProps}
                      headers={headers}
                      processToStage={processToStage}
                      cancelProcessing={cancelProcessing}
                      downloadAsset={downloadAsset}
                      getStageInfo={getStageInfo}
                    />
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
