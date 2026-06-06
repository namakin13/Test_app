import { renderHook, act } from "@testing-library/react";
import { vi, describe, it, expect, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useManualGames } from "../useManualGames";

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
}));

const mockInvoke = vi.mocked(invoke);
const mockOpen = vi.mocked(open);

beforeEach(() => {
  mockInvoke.mockReset();
  mockOpen.mockReset();
});

describe("useManualGames", () => {
  // ===== fetchManualGames =====

  describe("fetchManualGames", () => {
    it("成功時: games が更新され loading が false になる", async () => {
      const mockGames = [
        { id: "manual_1", name: "My Game", exe_path: "C:\\Games\\g.exe", category: "game" },
      ];
      mockInvoke.mockResolvedValueOnce(mockGames);

      const { result } = renderHook(() => useManualGames());

      await act(async () => {
        await result.current.fetchManualGames();
      });

      expect(result.current.games).toEqual(mockGames);
      expect(result.current.loading).toBe(false);
      expect(result.current.error).toBeNull();
    });

    it("失敗時: error が設定される", async () => {
      mockInvoke.mockRejectedValueOnce(new Error("read failed"));

      const { result } = renderHook(() => useManualGames());

      await act(async () => {
        await result.current.fetchManualGames();
      });

      expect(result.current.error).toContain("read failed");
    });
  });

  // ===== addManualGameViaDialog =====

  describe("addManualGameViaDialog", () => {
    it("ファイル選択後: ファイル名を初期名として add_manual_game を呼び再取得する（既定はgame）", async () => {
      mockOpen.mockResolvedValueOnce("C:\\Games\\Cool Game.exe");
      mockInvoke.mockResolvedValueOnce(undefined); // add_manual_game
      mockInvoke.mockResolvedValueOnce([]);        // fetchManualGames

      const { result } = renderHook(() => useManualGames());

      await act(async () => {
        await result.current.addManualGameViaDialog();
      });

      expect(mockInvoke).toHaveBeenCalledWith("add_manual_game", {
        name: "Cool Game",
        exePath: "C:\\Games\\Cool Game.exe",
        category: "game",
      });
    });

    it("category=app を指定するとアプリとして add_manual_game を呼ぶ", async () => {
      mockOpen.mockResolvedValueOnce("C:\\Apps\\Discord.exe");
      mockInvoke.mockResolvedValueOnce(undefined); // add_manual_game
      mockInvoke.mockResolvedValueOnce([]);        // fetchManualGames

      const { result } = renderHook(() => useManualGames());

      await act(async () => {
        await result.current.addManualGameViaDialog("app");
      });

      expect(mockInvoke).toHaveBeenCalledWith("add_manual_game", {
        name: "Discord",
        exePath: "C:\\Apps\\Discord.exe",
        category: "app",
      });
    });

    it("ダイアログがキャンセルされた場合: add_manual_game を呼ばない", async () => {
      mockOpen.mockResolvedValueOnce(null);

      const { result } = renderHook(() => useManualGames());

      await act(async () => {
        await result.current.addManualGameViaDialog();
      });

      expect(mockInvoke).not.toHaveBeenCalled();
    });

    it("追加失敗時: error が設定される", async () => {
      mockOpen.mockResolvedValueOnce("C:\\Games\\g.exe");
      mockInvoke.mockRejectedValueOnce(new Error("対応していない拡張子です"));

      const { result } = renderHook(() => useManualGames());

      await act(async () => {
        await result.current.addManualGameViaDialog();
      });

      expect(result.current.error).toContain("対応していない拡張子");
    });
  });

  // ===== removeManualGame =====

  describe("removeManualGame", () => {
    it("削除後に再取得する", async () => {
      mockInvoke.mockResolvedValueOnce(undefined); // remove_manual_game
      mockInvoke.mockResolvedValueOnce([]);        // fetchManualGames

      const { result } = renderHook(() => useManualGames());

      await act(async () => {
        await result.current.removeManualGame("manual_1");
      });

      expect(mockInvoke).toHaveBeenCalledWith("remove_manual_game", { id: "manual_1" });
      expect(mockInvoke).toHaveBeenCalledWith("get_manual_games");
    });
  });

  // ===== launchManualGame =====

  describe("launchManualGame", () => {
    it("id を渡して launch_manual_game を呼ぶ", async () => {
      mockInvoke.mockResolvedValueOnce(undefined);

      const { result } = renderHook(() => useManualGames());

      await act(async () => {
        await result.current.launchManualGame("manual_1");
      });

      expect(mockInvoke).toHaveBeenCalledWith("launch_manual_game", { id: "manual_1" });
    });

    it("失敗時: error が設定される", async () => {
      mockInvoke.mockRejectedValueOnce(new Error("launch failed"));

      const { result } = renderHook(() => useManualGames());

      await act(async () => {
        await result.current.launchManualGame("manual_1");
      });

      expect(result.current.error).toContain("launch failed");
    });
  });
});
