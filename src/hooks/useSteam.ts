import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface SteamGame {
  name: string;
  appid: string;
  header_image_url: string;
}

export const useSteam = () => {
  const [games, setGames] = useState<SteamGame[]>([]);
  const [loading, setLoading] = useState(false);
  const [hasFetched, setHasFetched] = useState(false);
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

  const fetchGames = async () => {
    setLoading(true);
    try {
      const result: SteamGame[] = await invoke("get_steam_games");
      setGames(result || []);
    } catch (error) {
      console.error("Failed to fetch games:", error);
    } finally {
      setLoading(false);
    }
  };

  const fetchAll = async () => {
    setHasFetched(true);
    await Promise.all([fetchCustomIcons(), fetchGames()]);
  };

  const launchGame = async (appid: string) => {
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

  const launchSteam = async () => {
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

  const saveCustomIcon = async (appid: string, filePath: string) => {
    try {
      await invoke("save_custom_icon_from_path", { appid, filePath });
      await fetchCustomIcons();
    } catch (error) {
      console.error("Failed to save custom icon from path:", error);
    }
  };

  return {
    games,
    loading,
    hasFetched,
    launching,
    launchingSteam,
    customIcons,
    fetchAll,
    fetchGames,
    launchGame,
    launchSteam,
    saveCustomIcon,
  };
};
