import React, { useEffect } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { invoke } from "@tauri-apps/api/core";
import { Volume2, Speaker, Headset, Loader2, RotateCw, Gamepad2, Headphones, SkipBack, Play, SkipForward, VolumeX, Plus, X } from "lucide-react";
import { Icon } from "@iconify/react";
import steamIcon from "@iconify-icons/simple-icons/steam";
import { motion, AnimatePresence } from "framer-motion";
import { GameBanner } from "./components/GameBanner";
import { useAudio } from "./hooks/useAudio";
import { useSteam } from "./hooks/useSteam";
import { useManualGames } from "./hooks/useManualGames";
import "./index.css";

function App() {
  const [activeTab, setActiveTab] = React.useState<"audio" | "games">("audio");
  const [gameSubTab, setGameSubTab] = React.useState<"steam" | "other">("steam");

  const {
    devices, loading: audioLoading, switching, volume,
    fetchDevices, switchDevice, setVolume,
  } = useAudio();
  const {
    games, loading: gamesLoading, hasFetched, launching, launchingSteam,
    customIcons, fetchAll, fetchGames, launchGame, launchSteam, saveCustomIcon,
  } = useSteam();
  const {
    games: manualGames, loading: manualLoading, launching: manualLaunching,
    fetchManualGames, addManualGameViaDialog, removeManualGame, launchManualGame,
  } = useManualGames();

  const [browserMuted, setBrowserMuted] = React.useState(false);
  const hoveredGameIdRef = React.useRef<string | null>(null);

  const mediaBtnStyle: React.CSSProperties = {
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    width: "44px",
    height: "44px",
    background: "rgba(255,255,255,0.05)",
    border: "1px solid rgba(255,255,255,0.1)",
    borderRadius: "12px",
    color: "white",
    cursor: "pointer",
    transition: "all 0.2s ease",
  };

  useEffect(() => {
    fetchDevices();

    const preventDefault = (e: Event) => e.preventDefault();
    window.addEventListener("dragover", preventDefault);
    window.addEventListener("drop", preventDefault);

    let unlisten: (() => void) | null = null;
    getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === 'drop' && event.payload.paths.length > 0) {
        const path = event.payload.paths[0];
        if (hoveredGameIdRef.current) {
          saveCustomIcon(hoveredGameIdRef.current, path);
        }
      }
    }).then(u => { unlisten = u; });

    return () => {
      window.removeEventListener("dragover", preventDefault);
      window.removeEventListener("drop", preventDefault);
      if (unlisten) unlisten();
    };
  }, []);

  useEffect(() => {
    if (activeTab === "games" && !hasFetched) {
      fetchAll();
      fetchManualGames();
    }
  }, [activeTab, hasFetched]);

  const getAudioIcon = (name: string) => {
    const lowerName = name.toLowerCase();
    if (lowerName.includes("headset") || lowerName.includes("headphones") || lowerName.includes("イヤホン")) return <Headset size={20} />;
    if (lowerName.includes("speaker") || lowerName.includes("スピーカー")) return <Speaker size={20} />;
    return <Volume2 size={20} />;
  };

  return (
    <div className="container">
      <header className="header">
        <div className="header-top">
          <h1 className="title">Control Center</h1>
          <button
            className="steam-launch-button"
            onClick={launchSteam}
            disabled={launchingSteam}
            title="Launch Steam"
          >
            {launchingSteam ? <Loader2 size={24} className="animate-spin" /> : (
              <Icon icon={steamIcon} width="24" height="24" />
            )}
          </button>
        </div>

        <div className="tabs">
          <button
            className={`tab ${activeTab === "audio" ? "active" : ""}`}
            onClick={() => setActiveTab("audio")}
          >
            <Headphones size={18} />
            Audio
          </button>
          <button
            className={`tab ${activeTab === "games" ? "active" : ""}`}
            onClick={() => setActiveTab("games")}
          >
            <Gamepad2 size={18} />
            Games
          </button>
        </div>

        <button
          className="refresh-button"
          onClick={
            activeTab === "audio"
              ? fetchDevices
              : gameSubTab === "steam"
              ? fetchGames
              : fetchManualGames
          }
          disabled={
            activeTab === "audio"
              ? audioLoading
              : gameSubTab === "steam"
              ? gamesLoading
              : manualLoading
          }
        >
          <RotateCw
            size={18}
            className={
              (activeTab === "audio"
                ? audioLoading
                : gameSubTab === "steam"
                ? gamesLoading
                : manualLoading)
                ? "animate-spin"
                : ""
            }
          />
        </button>
      </header>

      <div className="content-area">
        <AnimatePresence mode="wait">
          {activeTab === "audio" ? (
            <motion.div
              key="audio-tab"
              initial={{ opacity: 0, x: -20 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: 20 }}
              transition={{ duration: 0.2 }}
            >
              {audioLoading ? (
                <div className="loading-container">
                  <div className="spinner"></div>
                  <p className="loading-text">Loading devices...</p>
                </div>
              ) : (
                <>
                {/* メディアコントロール */}
                <div style={{
                  display: "flex",
                  gap: "8px",
                  justifyContent: "center",
                  marginBottom: "16px",
                  padding: "12px",
                  background: "rgba(255,255,255,0.03)",
                  borderRadius: "16px",
                  border: "1px solid rgba(255,255,255,0.08)",
                }}>
                  <button onClick={() => invoke("media_prev")} title="前へ" style={mediaBtnStyle}>
                    <SkipBack size={18} />
                  </button>
                  <button onClick={() => invoke("media_play_pause")} title="再生/一時停止" style={mediaBtnStyle}>
                    <Play size={18} />
                  </button>
                  <button onClick={() => invoke("media_next")} title="次へ" style={mediaBtnStyle}>
                    <SkipForward size={18} />
                  </button>
                  <button
                    onClick={async () => {
                      const newMuted: boolean = await invoke("toggle_browser_mute");
                      setBrowserMuted(newMuted);
                    }}
                    title="ブラウザミュート"
                    style={{
                      ...mediaBtnStyle,
                      background: browserMuted ? "rgba(255,80,80,0.3)" : "rgba(255,255,255,0.05)",
                      borderColor: browserMuted ? "rgba(255,80,80,0.5)" : "rgba(255,255,255,0.1)",
                    }}
                  >
                    <VolumeX size={18} />
                  </button>
                </div>
                <div className="device-list">
                  <AnimatePresence mode="popLayout">
                    {devices.length > 0 ? (
                      devices.map((device) => (
                        <motion.div
                          key={device.id}
                          layout
                          initial={{ opacity: 0, y: 20 }}
                          animate={{ opacity: 1, y: 0 }}
                          exit={{ opacity: 0, scale: 0.95 }}
                          className={`device-item ${device.is_default ? "active" : ""}`}
                          onClick={() => switchDevice(device.id)}
                        >
                          <div className="device-row">
                            <div className="device-info">
                              <div className="device-icon">
                                {switching === device.id ? (
                                  <Loader2 size={20} className="animate-spin" />
                                ) : (
                                  getAudioIcon(device.name)
                                )}
                              </div>
                              <span className="device-name">{device.name}</span>
                            </div>
                            <div className="status-dot"></div>
                          </div>
                          {device.is_default && (
                            <div
                              className="volume-control"
                              onClick={(e) => e.stopPropagation()}
                              onMouseDown={(e) => e.stopPropagation()}
                            >
                              <Volume2 size={16} />
                              <input
                                type="range"
                                min={0}
                                max={100}
                                step={1}
                                value={volume}
                                onChange={(e) => setVolume(device.id, parseInt(e.target.value, 10))}
                                className="volume-slider"
                                style={{ ["--volume-fill" as string]: `${volume}%` }}
                                aria-label="音量"
                              />
                              <span className="volume-value">{volume}%</span>
                            </div>
                          )}
                        </motion.div>
                      ))
                    ) : (
                      <div className="no-items">No audio devices found.</div>
                    )}
                  </AnimatePresence>
                </div>
                </>
              )}
            </motion.div>
          ) : (
            <motion.div
              key="games-tab"
              initial={{ opacity: 0, x: 20 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: -20 }}
              transition={{ duration: 0.2 }}
            >
              <div className="sub-tabs">
                <button
                  className={`sub-tab ${gameSubTab === "steam" ? "active" : ""}`}
                  onClick={() => setGameSubTab("steam")}
                >
                  <Icon icon={steamIcon} width="16" height="16" />
                  Steam
                </button>
                <button
                  className={`sub-tab ${gameSubTab === "other" ? "active" : ""}`}
                  onClick={() => setGameSubTab("other")}
                >
                  <Gamepad2 size={16} />
                  その他
                </button>
              </div>

              {gameSubTab === "steam" ? (
                gamesLoading ? (
                  <div className="loading-container">
                    <div className="spinner"></div>
                    <p className="loading-text">Loading Steam library...</p>
                  </div>
                ) : (
                  <div className="game-list">
                    <AnimatePresence mode="popLayout">
                      {games.length > 0 ? (
                        games.map((game) => (
                          <motion.div
                            key={game.appid}
                            layout
                            initial={{ opacity: 0, scale: 0.8 }}
                            animate={{ opacity: 1, scale: 1 }}
                            exit={{ opacity: 0, scale: 0.8 }}
                            className="game-card"
                            onClick={() => launchGame(game.appid)}
                            onMouseEnter={() => { hoveredGameIdRef.current = game.appid; }}
                            onMouseLeave={() => { if (hoveredGameIdRef.current === game.appid) hoveredGameIdRef.current = null; }}
                            title={game.name}
                          >
                            <GameBanner game={game} customIcon={customIcons[game.appid]} />
                            {launching === game.appid && (
                              <div className="game-launching-overlay">
                                <Loader2 size={24} className="animate-spin" />
                              </div>
                            )}
                          </motion.div>
                        ))
                      ) : (
                        <div className="no-items">No Steam games found.</div>
                      )}
                    </AnimatePresence>
                  </div>
                )
              ) : manualLoading ? (
                <div className="loading-container">
                  <div className="spinner"></div>
                  <p className="loading-text">Loading games...</p>
                </div>
              ) : (
                <div className="game-list">
                  <AnimatePresence mode="popLayout">
                    {manualGames.map((game) => (
                      <motion.div
                        key={game.id}
                        layout
                        initial={{ opacity: 0, scale: 0.8 }}
                        animate={{ opacity: 1, scale: 1 }}
                        exit={{ opacity: 0, scale: 0.8 }}
                        className="game-card"
                        onClick={() => launchManualGame(game.id)}
                        onMouseEnter={() => { hoveredGameIdRef.current = game.id; }}
                        onMouseLeave={() => { if (hoveredGameIdRef.current === game.id) hoveredGameIdRef.current = null; }}
                        title={game.name}
                      >
                        <GameBanner
                          game={{ name: game.name, appid: game.id, header_image_url: "" }}
                          customIcon={customIcons[game.id]}
                          manual
                        />
                        <button
                          className="game-remove-button"
                          title="削除"
                          onClick={(e) => {
                            e.stopPropagation();
                            removeManualGame(game.id);
                          }}
                        >
                          <X size={14} />
                        </button>
                        {manualLaunching === game.id && (
                          <div className="game-launching-overlay">
                            <Loader2 size={24} className="animate-spin" />
                          </div>
                        )}
                      </motion.div>
                    ))}
                    {/* 追加カード */}
                    <motion.div
                      key="__add_manual__"
                      layout
                      initial={{ opacity: 0, scale: 0.8 }}
                      animate={{ opacity: 1, scale: 1 }}
                      exit={{ opacity: 0, scale: 0.8 }}
                      className="game-card game-add-card"
                      onClick={addManualGameViaDialog}
                      title="ゲームを追加"
                    >
                      <Plus size={36} opacity={0.5} />
                    </motion.div>
                  </AnimatePresence>
                </div>
              )}
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    </div>
  );
}

export default App;
