import { ReactNode, useEffect } from "react";
import { CurrentStage } from "../../../../utils/schema";
import { ProgressIndicator, ProgressStep, Stack } from "@carbon/react";
import { useStore } from "../../../../utils/store";
import { UploadStage } from "./upload";
import { SeparateStage } from "./separate";
import { TranscribeStage } from "./tanscribe";

type StageConfig = {
  label: string;
  component: ReactNode;
  complete: boolean;
};

export const Stages = () => {
  const { currentStage, setCurrentStage, uploadedFile, separationProgress } =
    useStore();

  useEffect(() => {
    console.log("sp", separationProgress);
  }, [separationProgress]);

  const stages: Record<CurrentStage, StageConfig> = {
    upload: {
      label: "upload a song",
      component: <UploadStage />,
      complete: Boolean(uploadedFile),
    },
    separate: {
      label: "separate vocals from piano",
      component: <SeparateStage />,
      complete:
        separationProgress?.progress == 100 || currentStage === "transcribe",
    },
    transcribe: {
      label: "transcribe to sheet music",
      component: <TranscribeStage />,
      complete: false,
    },
  };

  return (
    <Stack orientation="vertical" style={{ width: "25rem", height: "20rem" }}>
      <ProgressIndicator>
        {Object.keys(stages).map((stage) => {
          const stg = stage as CurrentStage;

          return (
            <ProgressStep
              key={stg}
              current={currentStage === stg}
              label={stg}
              secondaryLabel={stages[stg].label}
              description={stages[stg].label}
              onClick={() => setCurrentStage(stg)}
              complete={stages[stg].complete}
            />
          );
        })}
      </ProgressIndicator>
      <div style={{ marginTop: "5rem" }}>{stages[currentStage].component}</div>
    </Stack>
  );
};
