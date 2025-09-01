import { useEffect } from "react";
import { getAppConfig } from "./utils/getAppConfig";
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
