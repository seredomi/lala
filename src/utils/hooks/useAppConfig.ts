import { useRecoilState, useRecoilValue } from "recoil";
import { appConfigState, loadingState } from "../state";
import { useEffect } from "react";
import { ToastNotification } from "@carbon/react";

import { invoke } from "@tauri-apps/api/core";
import { AppConfig } from "../types";
import { toast } from "sonner";

export const getAppConfig = async () => {
  let config: AppConfig;
  try {
    config = await invoke("get_app_config");
  } catch (e) {
    throw Error(`failed to fetch app config: ${e}`);
  }

  return config;
};

export const useAppConfig = () => {
  const [appConfig, setAppConfig] = useRecoilState(appConfigState);
  const [, setLoadingState] = useRecoilState(loadingState);
  const [, setErrorState] = useRecoilState(errorState);

  useEffect(() => {
    const fetchConfig = async () => {
      try {
        setLoadingState({
          title: "loading configuration",
          description: "fetching application settings...",
        });

        const config: AppConfig = await invoke("get_app_config");

        setLoadingState({
          title: "loading configuration",
          description: "configuration loaded successfully",
        });

        setAppConfig(config);
      } catch (err) {
        toast("failed to load config. falling back to client side definitions");
      }
      setLoadingState(null);
    };

    if (!appConfig) {
      fetchConfig();
    }
  }, [
    appConfig,
    setAppConfig,
    setLoadingState,
    clearLoading,
    setErrorState,
    clearError,
  ]);

  return { appConfig };
};
