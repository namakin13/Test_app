import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { Volume2, Speaker, Headset, Loader2, RotateCw, Gamepad2, Headphones } from "lucide-react";
import { Icon } from "@iconify/react";
import steamIcon from "@iconify-icons/simple-icons/steam";
import { motion, AnimatePresence } from "framer-motion";
import "./index.css";

interface AudioDevice {
  id: string;
  name: string;
  is_default: boolean;
}

interface SteamGame {
  name: string;
  appid: string;
  header_image_url: string;
}

const GameBanner = ({ game, customIcon }: { game: SteamGame, customIcon?: string }) => {
  const fallbacks = customIcon ? [
    customIcon,
    game.header_image_url,
    `https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/${game.appid}/header.jpg`,
    `https://cdn.cloudflare.steamstatic.com/steam/apps/${game.appid}/header.jpg`
  ] : [
    game.header_image_url,
    `https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/${game.appid}/header.jpg`,
    `https://cdn.cloudflare.steamstatic.com/steam/apps/${game.appid}/header.jpg`
  ];

  const [imgIndex, setImgIndex] = useState(0);
  const [isError, setIsError] = useState(false);

  useEffect(() => {
    setImgIndex(0);
    setIsError(false);
  }, [game.appid, customIcon]);

  return (
    <div className="game-banner-container">
      <img
        src={fallbacks[imgIndex]}
        alt={game.name}
        className="game-banner"
        style={isError ? { display: 'none' } : undefined}
        onError={() => {
          if (imgIndex < fallbacks.length - 1) {
            setImgIndex(prev => prev + 1);
          } else {
            setIsError(true);
          }
        }}
      />
      <div className={`game-banner-fallback ${isError ? '' : 'hidden'}`}>
        <Gamepad2 size={48} opacity={0.2} />
      </div>
    </div>
  );
};

function App() {
  const [activeTab, setActiveTab] = useState<"audio" | "games">("audio");

  // Audio State
  const [devices, setDevices] = useState<AudioDevice[]>([]);
  const [audioLoading, setAudioLoading] = useState(true);
  const [switching, setSwitching] = useState<string | null>(null);

  // Steam State
  const [games, setGames] = useState<SteamGame[]>([]);
  const [gamesLoading, setGamesLoading] = useState(false);
  const [hasFetchedGames, setHasFetchedGames] = useState(false);
  const [launching, setLaunching] = useState<string | null>(null);
  const [launchingSteam, setLaunchingSteam] = useState(false);
  const [customIcons, setCustomIcons] = useState<Record<string, string>>({});

  const fetchCustomIcons = async () => {
    try {
      const icons = await invoke<Record<string, string>>("get_custom_icons");
      setCustomIcons(icons || {});
    } catch (error) {
      console.error("Failed to fetch custom icons:", error);
    }
  };

  const fetchDevices = async () => {
    setAudioLoading(true);
    try {
      const result: AudioDevice[] = await invoke("get_audio_devices");
      setDevices(result);
    } catch (error) {
      console.error("Failed to fetch devices:", error);
    } finally {
      setAudioLoading(false);
    }
  };

  const fetchGames = async () => {
    setGamesLoading(true);
    try {
      const result: SteamGame[] = await invoke("get_steam_games");
      setGames(result || []);
    } catch (error) {
      console.error("Failed to fetch games:", error);
    } finally {
      setGamesLoading(false);
    }
  };

  useEffect(() => {
    fetchDevices();

    // Prevent default drag and drop behavior globally to avoid navigating to the dropped image
    const preventDefault = (e: Event) => e.preventDefault();
    window.addEventListener("dragover", preventDefault);
    window.addEventListener("drop", preventDefault);

    // Setup Tauri webview drop event listener
    let unlisten: (() => void) | null = null;
    getCurrentWebview().onDragDropEvent((event) => {
      // payload will be something like { type: 'drop', paths: ['C:\\Users\\...\\image.png'], position: { x, y } }
      if (event.payload.type === 'drop' && event.payload.paths.length > 0) {
        const path = event.payload.paths[0];
        // We need a way to know which game card the user hovered over.
        // For simplicity, we can store the "currently hovered game id" in a ref or state
        if (hoveredGameIdRef.current) {
          handleNativeDrop(path, hoveredGameIdRef.current);
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
    if (activeTab === "games" && !hasFetchedGames) {
      setHasFetchedGames(true);
      fetchCustomIcons();
      fetchGames();
    }
  }, [activeTab, hasFetchedGames]);

  const hoveredGameIdRef = React.useRef<string | null>(null);

  const handleNativeDrop = async (filePath: string, appid: string) => {
    try {
      await invoke("save_custom_icon_from_path", { appid, filePath });
      await fetchCustomIcons();
    } catch (error) {
      console.error("Failed to save custom icon from path:", error);
    }
  };

  const handleSwitchAudio = async (id: string) => {
    if (switching) return;
    setSwitching(id);
    try {
      await invoke("switch_audio_device", { id });
      await fetchDevices();
    } catch (error) {
      console.error("Failed to switch device:", error);
    } finally {
      setSwitching(null);
    }
  };

  const handleLaunchGame = async (appid: string) => {
    if (launching) return;
    setLaunching(appid);
    try {
      await invoke("launch_steam_game", { appid });
    } catch (error) {
      console.error("Failed to launch game:", error);
    } finally {
      setLaunching(null);
    }
  };

  const handleLaunchSteamApp = async () => {
    if (launchingSteam) return;
    setLaunchingSteam(true);
    try {
      await invoke("launch_steam_app");
    } catch (error) {
      console.error("Failed to launch Steam app:", error);
    } finally {
      setLaunchingSteam(false);
    }
  };

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
            onClick={handleLaunchSteamApp}
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
                          onClick={() => handleSwitchAudio(device.id)}
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
                      <div className="no-items">
                        No audio devices found.
                      </div>
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
                          onClick={() => handleLaunchGame(game.appid)}
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
                      <div className="no-items">
                        No Steam games found.
                      </div>
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
