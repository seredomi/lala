import {
  Header,
  HeaderName,
  HeaderGlobalBar,
  HeaderGlobalAction,
  Theme,
  Tooltip,
} from "@carbon/react";
import {
  CloseLarge,
  DownToBottom,
  Information,
  Maximize,
  Minimize,
  UpToTop,
} from "@carbon/icons-react";
import { exit } from "@tauri-apps/plugin-process";
import { useStore } from "../utils/store";
import { getCurrentWindow } from "@tauri-apps/api/window";

export const AppHeader = () => {
  const { setCurrentView } = useStore();
  const appWindow = getCurrentWindow();

  return (
    <Theme theme="g90">
      <Header data-tauri-drag-region>
        <HeaderName data-tauri-drag-region prefix="">
          <HeaderGlobalAction onClick={() => setCurrentView("main")}>
            <p style={{ color: "#f4f4f4" }}>lala</p>
          </HeaderGlobalAction>
        </HeaderName>
        <HeaderGlobalBar data-tauri-drag-region>
          <HeaderGlobalAction
            data-tauri-drag-region
            onClick={() => setCurrentView("about")}
          >
            <Information size={18} />
          </HeaderGlobalAction>
          <HeaderGlobalAction
            data-tauri-drag-region
            onClick={() => appWindow.minimize()}
          >
            <DownToBottom size={15} />
          </HeaderGlobalAction>
          <HeaderGlobalAction
            data-tauri-drag-region
            onClick={() => appWindow.maximize()}
          >
            <UpToTop size={15} />
          </HeaderGlobalAction>
          <Tooltip label="close app">
            <HeaderGlobalAction data-tauri-drag-region onClick={() => exit(0)}>
              <CloseLarge size={20} />
            </HeaderGlobalAction>
          </Tooltip>
        </HeaderGlobalBar>
      </Header>
    </Theme>
  );
};
