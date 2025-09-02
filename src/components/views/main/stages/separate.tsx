import { Button, Loading, ProgressBar, Stack } from "@carbon/react";
import { useStore } from "../../../../utils/store";
import { ArrowLeft, ArrowRight, Download, Renew } from "@carbon/icons-react";
import {
  abortSeparation,
  downloadStem,
  startSeparation,
} from "../../../../utils/separation";

export const SeparateStage = () => {
  const {
    setCurrentStage,
    separationProgress,
    setSeparationProgress,
    availableStems,
    downloadProgress,
  } = useStore();

  const proceedToTranscribe = () => {
    setCurrentStage("transcribe");
  };

  const handleDownloadStem = async (stemName: string) => {
    await downloadStem(stemName);
  };

  const isDownloading = downloadProgress && downloadProgress.progress !== 100;

  return (
    <Stack>
      <div style={{ minHeight: "14rem" }}>
        <h4 style={{ fontWeight: 800 }}>separate tracks</h4>

        {separationProgress ? (
          <div style={{ marginTop: "2rem" }}>
            {separationProgress.progress !== undefined && (
              <ProgressBar
                value={separationProgress.progress}
                max={100}
                status={
                  separationProgress.title !== "cancelled"
                    ? separationProgress.progress === 100
                      ? "finished"
                      : "active"
                    : "error"
                }
                label={separationProgress.title}
                helperText={`${separationProgress.progress}% - ${separationProgress.description}`}
              />
            )}

            {separationProgress.progress === 100 && (
              <div
                style={{
                  display: "flex",
                  justifyContent: "flex-end",
                  gap: "1rem",
                  marginTop: "2rem",
                  flexWrap: "wrap",
                }}
              >
                {availableStems.map((stemName) => (
                  <Button
                    key={stemName}
                    renderIcon={isDownloading ? Loading : Download}
                    kind="tertiary"
                    onClick={() => handleDownloadStem(stemName)}
                    disabled={isDownloading || false}
                  >
                    {stemName}
                  </Button>
                ))}
              </div>
            )}
          </div>
        ) : (
          <p style={{ marginTop: "2rem" }}>no separation in progress</p>
        )}
      </div>

      <div
        style={{
          display: "flex",
          flexDirection: "row",
          justifyContent: "space-between",
          alignItems: "center",
          maxHeight: "3rem",
        }}
      >
        {(separationProgress?.title === "cancelled" ||
          separationProgress?.progress === 100) && (
          <Button
            renderIcon={ArrowLeft}
            kind="tertiary"
            onClick={() => setCurrentStage("upload")}
          >
            change song
          </Button>
        )}

        {separationProgress?.progress !== 100 ? (
          separationProgress?.title === "cancelled" ? (
            <Button
              renderIcon={Renew}
              kind="primary"
              onClick={() => {
                setSeparationProgress(null);
                startSeparation();
              }}
            >
              retry
            </Button>
          ) : (
            <Button
              kind="danger"
              onClick={abortSeparation}
              style={{ marginLeft: "auto" }}
            >
              cancel
            </Button>
          )
        ) : (
          <Button
            renderIcon={ArrowRight}
            kind="primary"
            onClick={proceedToTranscribe}
            style={{ marginLeft: "auto" }}
          >
            transcribe
          </Button>
        )}
      </div>
    </Stack>
  );
};
