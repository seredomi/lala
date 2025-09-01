import { ReactNode } from "react";
import { CurrentStage } from "../../../../utils/schema";
import { AboutView } from "../../about-view";
import { ErrorView } from "../../error-view";
import { ProgressIndicator, ProgressStep, Stack } from "@carbon/react";
import { useStore } from "../../../../utils/store";
import { UploadStage } from "./upload";
import { SeparateStage } from "./separate";

type StageConfig = {
  label: string;
  component: ReactNode;
};

const stages: Record<CurrentStage, StageConfig> = {
  upload: {
    label: "upload a song",
    component: <UploadStage />,
  },
  separate: {
    label: "separate vocals from piano",
    component: <SeparateStage />,
  },
  transcribe: {
    label: "transcribe to sheet music",
    component: <ErrorView />,
  },
};

export const Stages = () => {
  const { currentStage, setCurrentStage } = useStore();

  return (
    <Stack orientation="vertical">
      <ProgressIndicator>
        {Object.keys(stages).map((stage, index) => {
          const stg = stage as CurrentStage;

          return (
            <ProgressStep
              key={stg}
              current={currentStage === stg}
              label={stg}
              secondaryLabel={stages[stg].label}
              description={stages[stg].label}
              onClick={() => setCurrentStage(stg)}
              complete={Object.keys(stages).indexOf(currentStage) > index}
            />
          );
        })}
      </ProgressIndicator>
      <div style={{ marginTop: "5rem" }}>{stages[currentStage].component}</div>
    </Stack>
  );
};
