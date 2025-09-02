import { Button, DismissibleTag, Stack } from "@carbon/react";
import { ArrowRight, DocumentAdd, DocumentImport } from "@carbon/icons-react";
import { useStore } from "../../../../utils/store";
import { open } from "@tauri-apps/plugin-dialog";
import { toast } from "../../../../utils/utils";
import { relaunch } from "@tauri-apps/plugin-process";
import { startSeparation } from "../../../../utils/separation";

export const UploadStage = () => {
  const { appConfig, selectedFilepath, setSelectedFilepath } = useStore();

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

      if (path) setSelectedFilepath(path);
    } catch (error) {
      console.error("unable to open file: ", error);
      toast({
        kind: "error",
        title: "unable to open file",
        subtitle: "try again, or try restarting the app",
        caption: String(error) || undefined,
        actionButtonLabel: "restart app",
        onActionButtonClick: () => relaunch(),
      });
    }
  };

  return (
    <Stack>
      <div style={{ minHeight: "14rem" }}>
        <h4 style={{ fontWeight: 800 }}>upload a song</h4>
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
            renderIcon={selectedFilepath ? DocumentImport : DocumentAdd}
            onClick={selectFile}
            style={{
              marginTop: "0.5rem",
              minWidth: "10rem",
            }}
            kind={selectedFilepath ? "tertiary" : "primary"}
          >
            {`${selectedFilepath ? "change" : "choose"} file`}
          </Button>
          {selectedFilepath && (
            <div>
              <DismissibleTag
                size="lg"
                onClose={() => setSelectedFilepath(null)}
                text={selectedFilepath.split("/").pop()}
                dismissTooltipLabel="remove song"
              />
            </div>
          )}
        </div>
      </div>

      <Button
        renderIcon={ArrowRight}
        disabled={!selectedFilepath}
        onClick={startSeparation}
        style={{
          justifySelf: "flex-end",
          maxHeight: "3rem",
        }}
      >
        separate
      </Button>
    </Stack>
  );
};
