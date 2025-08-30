import { useEffect, useState } from "react";
import {
  Button,
  Header,
  HeaderGlobalAction,
  HeaderGlobalBar,
  HeaderName,
  Theme,
} from "@carbon/react";
import { getAppConfig } from "./utils/getAppConfig";
import { AppConfig } from "./utils/types";
import { toast } from "./components/toast";
import { Close } from "@carbon/icons-react";
import { exit } from "@tauri-apps/plugin-process";
import { FileUploader } from "./components/file-uploader";

function App() {
  const [, setAppConfig] = useState<AppConfig | undefined>(undefined);

  useEffect(() => {
    const fetchConfig = async () => {
      try {
        const config = await getAppConfig();
        console.log("config", config);
        setAppConfig(config);
      } catch (error) {
        toast({
          kind: "info-square",
          title: "unable to load config from backend",
          subtitle: "falling back on clientside config",
          caption: error instanceof Error ? error.message : undefined,
          actionButtonLabel: "ok",
          actionCloses: true,
        });
      }
    };

    fetchConfig();
  }, []);

  return (
    <>
      <Theme theme="g90">
        <Header data-tauri-drag-region>
          <HeaderName
            data-tauri-drag-region
            prefix=""
            style={{ cursor: "default" }}
          >
            lala
          </HeaderName>
          <HeaderGlobalBar data-tauri-drag-region>
            <HeaderGlobalAction data-tauri-drag-region onClick={() => exit(0)}>
              <Close size={20} />
            </HeaderGlobalAction>
          </HeaderGlobalBar>
        </Header>
      </Theme>
      <Button
        kind="tertiary"
        onClick={() =>
          toast({
            kind: "info-square",
            actionButtonLabel: "Dismiss",
            title: "Hey!",
            subtitle: "Something happened",
            hideCloseButton: true,
            onActionButtonClick: () => console.log("clicked"),
            actionCloses: true,
          })
        }
      >
        Test toast
      </Button>
    </>
  );
}

export default App;
