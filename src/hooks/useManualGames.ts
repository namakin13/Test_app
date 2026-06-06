import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

export type ManualCategory = "game" | "app";

export interface ManualGame {
  id: string;
  name: string;
  exe_path: string;
  category: ManualCategory;
}

export const useManualGames = () => {
  const [games, setGames] = useState<ManualGame[]>([]);
  const [loading, setLoading] = useState(false);
  const [launching, setLaunching] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const fetchManualGames = async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<ManualGame[]>("get_manual_games");
      setGames(result || []);
    } catch (err) {
      console.error("Failed to fetch manual games:", err);
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  /** ファイル選択ダイアログを開いて手動エントリ（ゲーム/アプリ）を追加する */
  const addManualGameViaDialog = async (category: ManualCategory = "game") => {
    setError(null);
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        filters: [
          { name: "実行ファイル / ショートカット", extensions: ["exe", "lnk", "bat", "cmd", "url"] },
        ],
      });
      if (!selected || typeof selected !== "string") return;

      // パス区切りからファイル名（拡張子なし）を初期名として抽出
      const fileName = selected.split(/[\\/]/).pop() ?? "";
      const defaultName = fileName.replace(/\.[^.]+$/, "");

      await invoke<ManualGame>("add_manual_game", {
        name: defaultName,
        exePath: selected,
        category,
      });
      await fetchManualGames();
    } catch (err) {
      console.error("Failed to add manual game:", err);
      setError(String(err));
    }
  };

  const removeManualGame = async (id: string) => {
    setError(null);
    try {
      await invoke("remove_manual_game", { id });
      await fetchManualGames();
    } catch (err) {
      console.error("Failed to remove manual game:", err);
      setError(String(err));
    }
  };

  const launchManualGame = async (id: string) => {
    if (launching) return;
    setLaunching(id);
    try {
      await invoke("launch_manual_game", { id });
    } catch (err) {
      console.error("Failed to launch manual game:", err);
      setError(String(err));
    } finally {
      setLaunching(null);
    }
  };

  return {
    games,
    loading,
    launching,
    error,
    fetchManualGames,
    addManualGameViaDialog,
    removeManualGame,
    launchManualGame,
  };
};
