import {
  ActionableNotification,
  Button,
  Callout,
  DismissibleTag,
  FileUploader,
  Stack,
  Tag,
} from "@carbon/react";
import { AudioFileUploader } from "../file-uploader";
import {
  ArrowRight,
  DocumentAdd,
  DocumentImport,
  FileStorage,
  NextOutline,
  Restart,
  Upload,
  ViewNext,
} from "@carbon/icons-react";
import { useStore } from "../../../../utils/store";
import { useEffect } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { toast } from "../../../../utils/utils";

export const UploadStage = () => {
  const { setCurrentStage, appConfig, uploadedFile, setUploadedFile } =
    useStore();

  const selectFile = async () => {
    try {
      const path = await open({
        title: "choose a song",
        multiple: false,
        directory: false,
        filters: [
          {
            name: "file type",
            extensions: appConfig?.file_upload?.permitted_file_extensions || [],
          },
        ],
      });

      if (path) setUploadedFile(path);
    } catch (error) {
      console.error("unable to open file: ", error);
      toast({
        kind: "error",
        title: "unable to open file",
        subtitle: error?.message || "maybe try again",
      });
    }
  };

  return (
    <Stack style={{ minHeight: "20rem" }}>
      <div style={{ minHeight: "15rem" }}>
        <h4 style={{ fontWeight: 800 }}>upload audio file</h4>
        <p
          style={{ fontStyle: "italic", fontSize: 14, marginTop: "1rem" }}
        >{`max file size: ${appConfig?.file_upload?.max_file_size_mb || ""} MB`}</p>
        <p
          style={{ fontStyle: "italic", fontSize: 14, marginBottom: "0.5rem" }}
        >
          {`supported file types: ${(
            appConfig?.file_upload?.permitted_file_extensions || []
          )
            .map((str) => `.${str} `)
            .join("")} `}
        </p>
        <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
          <Button
            renderIcon={uploadedFile ? DocumentImport : DocumentAdd}
            onClick={selectFile}
            style={{
              marginTop: "0.5rem",
              minWidth: "10rem",
            }}
            kind={uploadedFile ? "tertiary" : "primary"}
          >
            {`${uploadedFile ? "change" : "choose"} song`}
          </Button>
          {uploadedFile && (
            <DismissibleTag
              size="lg"
              onClose={() => setUploadedFile(null)}
              text={uploadedFile.split("/").pop()}
              dismissTooltipLabel="remove song"
            />
          )}
        </div>
      </div>

      <Button
        renderIcon={ArrowRight}
        disabled={!uploadedFile}
        onClick={() => setCurrentStage("separate")}
        style={{
          justifySelf: "flex-end",
          alignSelf: "fex-end",
          maxHeight: "3rem",
        }}
      >
        next
      </Button>
    </Stack>
  );
};
