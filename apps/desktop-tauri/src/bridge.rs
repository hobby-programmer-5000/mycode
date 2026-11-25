use crate::backend::LocalBootstrapInfo;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeSeed {
    pub platform: &'static str,
    pub locale: String,
    pub languages: Vec<String>,
    pub local_environment_enabled: bool,
    pub bootstrap: LocalBootstrapInfo,
    pub version: &'static str,
}

/// Builds the Tauri `initialization_script` that exposes `window.desktopBridge` matching
/// `packages/contracts/src/ipc.ts` (`DesktopBridge`) before any web module executes.
pub fn build_initialization_script(seed: &BridgeSeed) -> String {
    let seed_json = serde_json::to_string(seed).unwrap_or_else(|_| "{}".to_string());

    format!(
        r#"(function () {{
  const SEED = {seed_json};
  const invoke = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args || {{}});
  const noopUnsub = () => () => {{}};
  const asyncVoid = () => Promise.resolve(undefined);
  const asyncNull = () => Promise.resolve(null);
  const asyncFalse = () => Promise.resolve(false);
  const asyncEmpty = () => Promise.resolve([]);

  let localEnabled = SEED.localEnvironmentEnabled !== false;
  let isFullscreen = false;

  const updateState = () => invoke("get_update_state");
  const serverExposureState = () => invoke("get_server_exposure_state");
  const wslState = () => invoke("get_wsl_state");
  const cliState = () => invoke("get_cli_command_state");

  const bridge = {{
    getAppBranding: () => ({{
      baseName: "MyCode",
      stageLabel: "Beta",
      displayName: "MyCode",
    }}),
    getDatabaseMigrationProgress: asyncNull,
    getRelayTelemetryEnabled: () => false,
    getPathForFile: (file) => (file && typeof file.path === "string" ? file.path : ""),
    getClientPlatform: () => SEED.platform,
    restartAudioService: asyncFalse,
    setNotificationBadge: asyncVoid,
    onNotificationBadgeClear: noopUnsub,
    onTrackpadScrollBegin: noopUnsub,
    onTrackpadScrollEnd: noopUnsub,
    revealWindow: () => invoke("reveal_window"),
    getSystemLocale: () => SEED.locale,
    getPreferredSystemLanguages: () => SEED.languages.slice(),
    getPrimaryPasteEnabled: () => true,
    getLocalEnvironmentBootstraps: () => (localEnabled ? [SEED.bootstrap] : []),
    getLocalEnvironmentEnabled: () => localEnabled,
    setLocalEnvironmentEnabled: async (enabled) => {{
      localEnabled = Boolean(enabled);
      await invoke("set_local_environment_enabled", {{ enabled: localEnabled }});
    }},
    getHostEnvironmentId: () => invoke("get_host_environment_id"),
    getLocalEnvironmentBearerToken: () => invoke("get_local_environment_bearer_token"),
    getClientSettings: () => invoke("get_client_settings"),
    setClientSettings: (settings) => invoke("set_client_settings", {{ settings }}),
    getConnectionCatalog: () => invoke("get_connection_catalog"),
    setConnectionCatalog: (catalog) => invoke("set_connection_catalog", {{ catalog }}),
    discoverSshHosts: asyncEmpty,
    resolveSshHost: () => Promise.reject(new Error("SSH is disabled in offline MyCode builds.")),
    requestSnapShotPermissions: asyncVoid,
    getSnapShotState: () => invoke("get_snap_shot_state"),
    setupSnapShot: asyncVoid,
    previewSnapShotConfig: asyncNull,
    applySnapShotConfig: () => Promise.resolve({{ backupPath: null, warning: null }}),
    checkSnapShotShortcut: () => Promise.resolve({{ available: false, message: null }}),
    setSnapShotShortcutSuppressed: asyncVoid,
    listPendingSnapShots: asyncEmpty,
    readSnapShot: () => Promise.reject(new Error("SnapShot unavailable")),
    setSnapShotAnimationDestination: asyncVoid,
    dismissSnapShotAnimation: asyncVoid,
    acknowledgeSnapShot: asyncVoid,
    ensureSshEnvironment: () => Promise.reject(new Error("SSH is disabled in offline MyCode builds.")),
    disconnectSshEnvironment: asyncVoid,
    fetchSshEnvironmentDescriptor: () => Promise.reject(new Error("SSH is disabled in offline MyCode builds.")),
    bootstrapSshBearerSession: () => Promise.reject(new Error("SSH is disabled in offline MyCode builds.")),
    fetchSshSessionState: () => Promise.reject(new Error("SSH is disabled in offline MyCode builds.")),
    onSshPasswordPrompt: noopUnsub,
    onSshPasswordPromptSettled: noopUnsub,
    resolveSshPasswordPrompt: asyncVoid,
    getServerExposureState: serverExposureState,
    setServerExposureMode: () => serverExposureState(),
    setTailscaleServeEnabled: () => serverExposureState(),
    getAdvertisedEndpoints: asyncEmpty,
    getWslState: wslState,
    setWslBackendEnabled: () => wslState(),
    setWslDistro: () => wslState(),
    setWslOnly: () => wslState(),
    pickFolder: (options) => invoke("pick_folder", {{ options: options || null }}),
    toLocalBackendPath: (backendId, path) =>
      invoke("to_local_backend_path", {{ backendId, path }}),
    pickProjectFavicon: (initialPath) =>
      invoke("pick_project_favicon", {{ initialPath: initialPath || null }}),
    pickThemeFiles: () => invoke("pick_theme_files"),
    setTheme: (theme) => invoke("set_theme", {{ theme }}),
    showContextMenu: asyncNull,
    receiveProviderAuthCallback: () =>
      Promise.reject(new Error("External auth callbacks are disabled in offline builds.")),
    cancelProviderAuthCallback: asyncVoid,
    startFileDrag: asyncFalse,
    openExternal: (url) => invoke("open_external", {{ url }}),
    readMedia: (url) => invoke("read_media", {{ url }}),
    openSystemSettings: asyncFalse,
    checkSystemPermission: asyncFalse,
    probeRemoteEditors: asyncEmpty,
    pasteAsText: asyncVoid,
    onMenuAction: noopUnsub,
    onSnapShotEvent: noopUnsub,
    onQuitShortcut: noopUnsub,
    getWindowFullscreenState: () => isFullscreen,
    onWindowFullscreenStateChange: noopUnsub,
    getWindowZoomFactor: () => 1,
    getUpdateState: updateState,
    setUpdateChannel: () => updateState(),
    checkForUpdate: async () => ({{ checked: false, state: await updateState() }}),
    downloadUpdate: async () => ({{ accepted: false, completed: false, state: await updateState() }}),
    installUpdate: async () => ({{ accepted: false, completed: false, state: await updateState() }}),
    onUpdateState: noopUnsub,
    cliCommand: {{
      getState: cliState,
      install: cliState,
      uninstall: cliState,
      addToPath: cliState,
    }},
    webLinks: {{
      setReady: asyncVoid,
      onOpen: noopUnsub,
    }},
    appActivation: {{
      setReady: asyncVoid,
      complete: asyncVoid,
      onRequest: noopUnsub,
    }},
    preview: {{
      setForwardedShortcuts: asyncVoid,
      createTab: asyncVoid,
      closeTab: asyncVoid,
      registerWebview: asyncVoid,
      navigate: asyncVoid,
      goBack: asyncVoid,
      goForward: asyncVoid,
      refresh: asyncVoid,
      stopLoading: asyncVoid,
      zoomIn: asyncVoid,
      zoomOut: asyncVoid,
      resetZoom: asyncVoid,
      setZoomFactor: asyncVoid,
      hardReload: asyncVoid,
      setColorScheme: asyncVoid,
      setAudioMuted: asyncVoid,
      findInPage: asyncVoid,
      stopFindInPage: asyncVoid,
      setShown: asyncVoid,
      setCaptureDrawn: asyncVoid,
      openDevTools: asyncVoid,
      clearCookies: asyncVoid,
      clearCache: asyncVoid,
      getPreviewConfig: () => Promise.resolve({{ partition: "persist:offline", preloadUrl: "", tunnelLoopback: false }}),
      onTunnelAccessRequest: noopUnsub,
      resolveTunnelAccess: asyncVoid,
      forwardPort: () => Promise.reject(new Error("Port forwarding is disabled in offline mode.")),
      stopPortForward: asyncVoid,
      listPortForwards: asyncEmpty,
      sharesLoopback: () => Promise.resolve(true),
      onPortForwards: noopUnsub,
      listBrowserImportSources: asyncEmpty,
      importBrowserCookies: () => Promise.resolve({{ importedCount: 0 }}),
      setAnnotationTheme: asyncVoid,
      setAnnotationSendEnabled: asyncVoid,
      annotationVoice: {{
        onEvent: noopUnsub,
        update: asyncVoid,
      }},
      pickElement: asyncNull,
      cancelPickElement: asyncVoid,
      captureScreenshot: () => Promise.reject(new Error("Preview capture unavailable")),
      revealArtifact: asyncVoid,
      copyArtifactToClipboard: asyncVoid,
      pictureInPicture: {{
        open: asyncVoid,
        close: asyncVoid,
      }},
      recording: {{
        onInput: noopUnsub,
        startScreencast: asyncVoid,
        stopScreencast: asyncVoid,
        save: () => Promise.reject(new Error("Preview recording unavailable")),
      }},
      onStateChange: noopUnsub,
      onPointerEvent: noopUnsub,
      onOpenLink: noopUnsub,
      onFindResult: noopUnsub,
    }},
    rustEngineInfo: () => invoke("rust_engine_info"),
  }};

  document.addEventListener("fullscreenchange", () => {{
    isFullscreen = Boolean(document.fullscreenElement);
  }});

  Object.defineProperty(window, "desktopBridge", {{
    value: bridge,
    writable: false,
    configurable: false,
  }});
}})();
"#
    )
}
