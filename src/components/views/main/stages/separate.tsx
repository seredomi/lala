import { Button, ProgressBar, Stack } from "@carbon/react";
import { useStore } from "../../../../utils/store";
import { ArrowLeft, ArrowRight, Download, Renew } from "@carbon/icons-react";
import { abortSeparation, startSeparation } from "../../../../utils/separation";

export const SeparateStage = () => {
  const { setCurrentStage, separationProgress, setSeparationProgress } =
    useStore();

  const proceedToTranscribe = () => {
    setCurrentStage("transcribe");
  };

  const downloadSongs = () => {
    // TODO: Implement download functionality
    console.log("Downloading separated songs...");
  };

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
                  marginTop: "2rem",
                }}
              >
                <Button
                  renderIcon={Download}
                  kind="tertiary"
                  onClick={downloadSongs}
                >
                  download tracks
                </Button>
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
              kind="danger--tertiary"
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
