import {
  Header,
  HeaderName,
  HeaderGlobalBar,
  HeaderGlobalAction,
  Theme,
} from "@carbon/react";
import { Close, InformationSquare } from "@carbon/icons-react";
import { exit } from "@tauri-apps/plugin-process";
import { useStore } from "../utils/store";

export const AppHeader = () => {
  const { setCurrentView } = useStore();

  return (
    <Theme theme="g90">
      <Header data-tauri-drag-region>
        <HeaderName
          data-tauri-drag-region
          prefix=""
          style={{ cursor: "grab" }}
          // onClick=
        >
          <HeaderGlobalAction onClick={() => setCurrentView("main")}>
            <p style={{ color: "#f4f4f4" }}>lala</p>
          </HeaderGlobalAction>
        </HeaderName>
        <HeaderGlobalBar data-tauri-drag-region>
          <HeaderGlobalAction
            data-tauri-drag-region
            onClick={() => setCurrentView("about")}
          >
            <InformationSquare size={20} />
          </HeaderGlobalAction>
          <HeaderGlobalAction data-tauri-drag-region onClick={() => exit(0)}>
            <Close size={20} />
          </HeaderGlobalAction>
        </HeaderGlobalBar>
      </Header>
    </Theme>
  );
};
