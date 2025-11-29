import { ReactNode } from "react";
import { CurrentView } from "../utils/schema";
import { MainView } from "./views/main-view";
import { AboutView } from "./views/about-view";
import { ErrorView } from "./views/error-view";
import { useStore } from "../utils/store";
import { Theme } from "@carbon/react";

export const viewMap: Record<CurrentView, ReactNode> = {
  main: <MainView />,
  about: <AboutView />,
  error: <ErrorView />,
};

export const AppView = () => {
  const { currentView } = useStore();

  return (
    <Theme theme="white">
      <div
        style={{
          overflowY: "auto",
          overscrollBehavior: "contain",
          height: "100vh",
          paddingTop: "5rem",
          display: "flex",
          flexDirection: "column",
          justifyItems: "center",
          alignItems: "center",
        }}
      >
        {viewMap[currentView]}
      </div>
    </Theme>
  );
};
