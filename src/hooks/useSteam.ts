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
  const [error, setError] = useState<string | null>(null);

  const fetchCustomIcons = async () => {
    try {
      const icons = await invoke<Record<string, string>>("get_custom_icons");
      setCustomIcons(icons || {});
    } catch (err) {
      console.error("Failed to fetch custom icons:", err);
      // 修正: エラーを error ステートに格納してUIに伝播できるようにする
      setError(String(err));
    }
  };

  const fetchGames = async () => {
    setLoading(true);
    setError(null);
    try {
      const result: SteamGame[] = await invoke("get_steam_games");
      setGames(result || []);
    } catch (err) {
      console.error("Failed to fetch games:", err);
      // 修正: エラーを error ステートに格納してUIに伝播できるようにする
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const fetchAll = async () => {
    setHasFetched(true);
    setError(null);
    await Promise.all([fetchCustomIcons(), fetchGames()]);
  };

  const launchGame = async (appid: string) => {
    if (launching) return;
    setLaunching(appid);
    try {
      await invoke("launch_steam_game", { appid });
    } catch (err) {
      console.error("Failed to launch game:", err);
      setError(String(err));
    } finally {
      setLaunching(null);
    }
  };

  const launchSteam = async () => {
    if (launchingSteam) return;
    setLaunchingSteam(true);
    try {
      await invoke("launch_steam_app");
    } catch (err) {
      console.error("Failed to launch Steam app:", err);
      setError(String(err));
    } finally {
      setLaunchingSteam(false);
    }
  };

  const saveCustomIcon = async (appid: string, filePath: string) => {
    try {
      await invoke("save_custom_icon_from_path", { appid, filePath });
      await fetchCustomIcons();
    } catch (err) {
      console.error("Failed to save custom icon from path:", err);
      setError(String(err));
    }
  };

  return {
    games,
    loading,
    hasFetched,
    launching,
    launchingSteam,
    customIcons,
    error,
    fetchAll,
    fetchGames,
    launchGame,
    launchSteam,
    saveCustomIcon,
  };
};
