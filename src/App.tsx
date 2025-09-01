import { useEffect } from "react";
import {
  Button,
  Header,
  HeaderGlobalAction,
  HeaderGlobalBar,
  HeaderName,
  Stack,
  Theme,
  Tile,
} from "@carbon/react";
import { getAppConfig } from "./utils/getAppConfig";
import { Close } from "@carbon/icons-react";
import { exit } from "@tauri-apps/plugin-process";
import { FileUploader } from "./components/views/main/file-uploader";
import { useStore } from "./utils/store";
import { AppHeader } from "./components/app-header";
import { AppView } from "./components/app-view";

function App() {
  const { setAppConfig } = useStore();

  useEffect(() => {
    const fetchConfig = async () => {
      const config = await getAppConfig();
      setAppConfig(config);
    };

    fetchConfig();
  }, [setAppConfig]);

  return (
    <>
      <AppHeader />
      <AppView />
    </>
  );
}

export default App;
