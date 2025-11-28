import { useFiles } from "../../utils/use-files";
import { FileTable } from "../files-table";

export const OldMainView = () => {
  const {
    files,
    isLoading,
    uploadFile,
    processToStage,
    cancelProcessing,
    deleteFile,
    downloadAsset,
  } = useFiles();

  if (isLoading) {
    return <div>Loading...</div>;
  }

  return (
    <div>
      <button onClick={uploadFile}>Upload File</button>

      <table>
        <thead>
          <tr>
            <th>Filename</th>
            <th>Stems</th>
            <th>MIDI</th>
            <th>PDF</th>
            <th>Status</th>
            <th>Progress</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          {files.map((file) => (
            <tr key={file.id}>
              <td>{file.original_filename}</td>
              <td>{file.has_stems ? "✓" : "—"}</td>
              <td>{file.has_midi ? "✓" : "—"}</td>
              <td>{file.has_pdf ? "✓" : "—"}</td>
              <td>
                {file.current_status || "ready"}
                {file.current_asset_type && ` (${file.current_asset_type})`}
              </td>
              <td>
                {file.current_progress && (
                  <div>
                    <div>{file.current_progress.title}</div>
                    <div className="text-sm text-gray-500">
                      {file.current_progress.description}
                    </div>
                  </div>
                )}
              </td>
              <td>
                {!file.current_status && (
                  <>
                    <button onClick={() => processToStage(file.id, "stems")}>
                      → Stems
                    </button>
                    <button onClick={() => processToStage(file.id, "midi")}>
                      → MIDI
                    </button>
                    <button onClick={() => processToStage(file.id, "pdf")}>
                      → PDF
                    </button>
                  </>
                )}

                {file.current_status === "processing" && (
                  <button onClick={() => cancelProcessing(file.id)}>
                    Cancel
                  </button>
                )}

                {file.has_stems && (
                  <button onClick={() => downloadAsset(file.id, "stem_piano")}>
                    ⬇ Piano
                  </button>
                )}

                {file.has_midi && (
                  <button onClick={() => downloadAsset(file.id, "midi")}>
                    ⬇ MIDI
                  </button>
                )}

                {file.has_pdf && (
                  <button onClick={() => downloadAsset(file.id, "pdf")}>
                    ⬇ PDF
                  </button>
                )}

                <button onClick={() => deleteFile(file.id)}>Delete</button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
};

export const MainView = () => {
  return <FileTable />;
};
