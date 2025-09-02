import { useEffect } from "react";
import { getAppConfig } from "./utils/getAppConfig";
import { useStore } from "./utils/store";
import { AppHeader } from "./components/app-header";
import { AppView } from "./components/app-view";
import { LoadingState } from "./utils/schema";
import { listen } from "@tauri-apps/api/event";

function App() {
  const { setAppConfig, setSeparationProgress, setDownloadProgress } =
    useStore();

  useEffect(() => {
    const fetchConfig = async () => {
      const config = await getAppConfig();
      setAppConfig(config);
    };
    fetchConfig();
  }, [setAppConfig]);

  useEffect(() => {
    const unlistenSeparation = listen<LoadingState>(
      "separation_progress",
      (event) => setSeparationProgress(event.payload),
    );

    const unlistenDownload = listen<LoadingState>(
      "download_progress",
      (event) => setDownloadProgress(event.payload),
    );

    return () => {
      unlistenSeparation.then((fn) => fn());
      unlistenDownload.then((fn) => fn());
    };
  }, [setSeparationProgress, setDownloadProgress]);

  return (
    <>
      <AppHeader />
      <AppView />
    </>
  );
}

export default App;
