import React, { useEffect } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { Volume2, Speaker, Headset, Loader2, RotateCw, Gamepad2, Headphones } from "lucide-react";
import { Icon } from "@iconify/react";
import steamIcon from "@iconify-icons/simple-icons/steam";
import { motion, AnimatePresence } from "framer-motion";
import { GameBanner } from "./components/GameBanner";
import { useAudio } from "./hooks/useAudio";
import { useSteam } from "./hooks/useSteam";
import "./index.css";

function App() {
  const [activeTab, setActiveTab] = React.useState<"audio" | "games">("audio");

  const { devices, loading: audioLoading, switching, fetchDevices, switchDevice } = useAudio();
  const {
    games, loading: gamesLoading, hasFetched, launching, launchingSteam,
    customIcons, fetchAll, fetchGames, launchGame, launchSteam, saveCustomIcon,
  } = useSteam();

  const hoveredGameIdRef = React.useRef<string | null>(null);

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
          onClick={activeTab === "audio" ? fetchDevices : fetchGames}
          disabled={activeTab === "audio" ? audioLoading : gamesLoading}
        >
          <RotateCw size={18} className={(activeTab === "audio" ? audioLoading : gamesLoading) ? "animate-spin" : ""} />
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
                        </motion.div>
                      ))
                    ) : (
                      <div className="no-items">No audio devices found.</div>
                    )}
                  </AnimatePresence>
                </div>
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
              {gamesLoading ? (
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
              )}
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    </div>
  );
}

export default App;
