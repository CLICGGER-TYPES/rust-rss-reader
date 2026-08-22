import React from "react";
import ReactDOM from "react-dom/client";
import { FluentProvider, webDarkTheme, webLightTheme } from "@fluentui/react-components";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { api } from "./api";
import { BUILD_ID, installErrorHandlers, log } from "./logger";
import { I18nProvider } from "./i18n";
import App from "./App";
import "./styles.css";

export type ThemeMode = "system" | "light" | "dark";

installErrorHandlers();
log("frontend boot", { build: BUILD_ID, userAgent: navigator.userAgent });
log("api.refresh invokes refresh with key fetch_full", { contract: "snake_case" });

function Root() {
  const [themeMode, setThemeMode] = React.useState<ThemeMode>("system");
  const [systemDark, setSystemDark] = React.useState(false);
  const [decorations, setDecorations] = React.useState(true);

  React.useEffect(() => {
    api.getSetting("theme").then((v) => {
      if (v === "light" || v === "dark" || v === "system") setThemeMode(v as ThemeMode);
    });
    api.getSetting("decorations").then((v) => {
      const d = v !== "0";
      setDecorations(d);
      getCurrentWindow().setDecorations(d).catch(() => {});
    });
  }, []);

  React.useEffect(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const update = () => setSystemDark(mq.matches);
    update();
    mq.addEventListener("change", update);
    return () => mq.removeEventListener("change", update);
  }, []);

  const dark = themeMode === "dark" || (themeMode === "system" && systemDark);

  const applyTheme = React.useCallback((mode: ThemeMode) => {
    setThemeMode(mode);
    api.setSetting("theme", mode);
  }, []);

  const applyDecorations = React.useCallback((d: boolean) => {
    setDecorations(d);
    api.setSetting("decorations", d ? "1" : "0");
    getCurrentWindow().setDecorations(d).catch(() => {});
  }, []);

  return (
    <FluentProvider theme={dark ? webDarkTheme : webLightTheme}>
      <I18nProvider>
        <App dark={dark} themeMode={themeMode} setThemeMode={applyTheme} decorations={decorations} setDecorations={applyDecorations} />
      </I18nProvider>
    </FluentProvider>
  );
}

ReactDOM.createRoot(document.getElementById("root")!).render(<Root />);
