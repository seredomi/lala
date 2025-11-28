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
  TableToolbarAction,
  TableToolbarContent,
  TableToolbarSearch,
} from "@carbon/react";
import { useFiles } from "../utils/use-files";
import {
  AddFilled,
  ArrowUpRight,
  FileStorage,
  DocumentAdd,
} from "@carbon/icons-react";

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
    {
      header: "status",
      key: "current_status",
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
                  <TableHeader {...getHeaderProps({ header })}>
                    {header.header}
                  </TableHeader>
                ))}
              </TableRow>
            </TableHead>
            <TableBody>
              {rows.length ? (
                rows.map((row) => (
                  <>
                    <TableExpandRow {...getRowProps({ row })}>
                      <TableSelectRow {...getSelectionProps({ row })} />
                      {row.cells.map((cell) => (
                        <TableCell {...getCellProps({ cell })}>
                          {cell.value}
                        </TableCell>
                      ))}
                    </TableExpandRow>
                    {row.isExpanded && (
                      <TableExpandedRow
                        {...getExpandedRowProps({ row })}
                        colSpan={headers.length + 2}
                      >
                        <p>ok</p>
                      </TableExpandedRow>
                    )}
                  </>
                ))
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
                        <p>click add audio to get started</p>
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
