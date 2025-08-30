import { invoke } from "@tauri-apps/api/core";
import { AppConfig } from "./types";

export const getAppConfig = async () => {
  let config: AppConfig;
  try {
    config = await invoke("get_app_config");
  } catch (e) {
    throw Error(`failed to fetch app config: ${e}`);
  }

  return config;
};
