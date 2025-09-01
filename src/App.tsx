import { useEffect } from "react";
import { getAppConfig } from "./utils/getAppConfig";
import { useStore } from "./utils/store";
import { AppHeader } from "./components/app-header";
import { AppView } from "./components/app-view";
import { LoadingState } from "./utils/schema";
import { listen } from "@tauri-apps/api/event";

function App() {
  const { setAppConfig, setSeparationProgress } = useStore();

  useEffect(() => {
    const fetchConfig = async () => {
      const config = await getAppConfig();
      setAppConfig(config);
    };
    fetchConfig();
  }, [setAppConfig]);

  useEffect(() => {
    const unlisten = listen<LoadingState>("separation_progress", (event) => {
      setSeparationProgress(event.payload);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [setSeparationProgress]);

  return (
    <>
      <AppHeader />
      <AppView />
    </>
  );
}

export default App;
