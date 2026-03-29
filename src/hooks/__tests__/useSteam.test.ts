import { renderHook, act } from "@testing-library/react";
import { vi, describe, it, expect, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { useSteam } from "../useSteam";

const mockInvoke = vi.mocked(invoke);

beforeEach(() => {
  mockInvoke.mockReset();
});

describe("useSteam", () => {
  // ===== fetchCustomIcons のテスト =====

  describe("fetchCustomIcons (fetchAll 経由)", () => {
    it("カスタムアイコン取得失敗時: customIcons は空のまま error が設定される", async () => {
      // get_custom_icons: 失敗
      mockInvoke.mockRejectedValueOnce(new Error("Icons fetch failed"));
      // get_steam_games: 成功
      mockInvoke.mockResolvedValueOnce([]);

      const { result } = renderHook(() => useSteam());

      await act(async () => {
        await result.current.fetchAll();
      });

      // 修正前: error は常に null のままだった
      // 修正後: エラーが error ステートに格納される
      expect(result.current.customIcons).toEqual({});
      expect(result.current.error).not.toBeNull();
      expect(result.current.error).toContain("Icons fetch failed");
    });

    it("カスタムアイコン取得成功時: customIcons が更新される", async () => {
      const mockIcons = { "440": "data:image/png;base64,abc" };
      mockInvoke.mockResolvedValueOnce(mockIcons); // get_custom_icons
      mockInvoke.mockResolvedValueOnce([]);          // get_steam_games

      const { result } = renderHook(() => useSteam());

      await act(async () => {
        await result.current.fetchAll();
      });

      expect(result.current.customIcons).toEqual(mockIcons);
      expect(result.current.error).toBeNull();
    });
  });

  // ===== fetchGames のテスト =====

  describe("fetchGames", () => {
    it("成功時: games が更新され loading が false になる", async () => {
      const mockGames = [
        { appid: "440", name: "Team Fortress 2", header_image_url: "https://example.com/440.jpg" },
      ];
      mockInvoke.mockResolvedValueOnce(mockGames);

      const { result } = renderHook(() => useSteam());

      await act(async () => {
        await result.current.fetchGames();
      });

      expect(result.current.games).toEqual(mockGames);
      expect(result.current.loading).toBe(false);
    });

    it("失敗時: games は空のまま error が設定される（Steam未インストール等）", async () => {
      mockInvoke.mockRejectedValueOnce(new Error("Steam not found"));

      const { result } = renderHook(() => useSteam());

      await act(async () => {
        await result.current.fetchGames();
      });

      expect(result.current.games).toEqual([]);
      expect(result.current.loading).toBe(false);
      expect(result.current.error).not.toBeNull();
      expect(result.current.error).toContain("Steam not found");
    });

    it("fetchAll は fetchCustomIcons と fetchGames を並行実行する", async () => {
      mockInvoke.mockResolvedValueOnce({ "730": "data:image/png;base64,xyz" }); // get_custom_icons
      mockInvoke.mockResolvedValueOnce([
        { appid: "730", name: "Counter-Strike 2", header_image_url: "https://example.com/730.jpg" },
      ]); // get_steam_games

      const { result } = renderHook(() => useSteam());

      await act(async () => {
        await result.current.fetchAll();
      });

      expect(result.current.hasFetched).toBe(true);
      expect(result.current.customIcons).toHaveProperty("730");
      expect(result.current.games).toHaveLength(1);
    });
  });

  // ===== launchGame のテスト =====

  describe("launchGame", () => {
    it("成功時: launching が null に戻る", async () => {
      mockInvoke.mockResolvedValueOnce(undefined);

      const { result } = renderHook(() => useSteam());

      await act(async () => {
        await result.current.launchGame("440");
      });

      expect(result.current.launching).toBeNull();
      expect(result.current.error).toBeNull();
    });

    it("失敗時: launching が null に戻り error が設定される", async () => {
      mockInvoke.mockRejectedValueOnce(new Error("Steam URI failed"));

      const { result } = renderHook(() => useSteam());

      await act(async () => {
        await result.current.launchGame("440");
      });

      expect(result.current.launching).toBeNull();
      expect(result.current.error).not.toBeNull();
    });

    it("起動中に再度 launchGame を呼んでも重複実行しない", async () => {
      let resolveLaunch!: () => void;
      mockInvoke.mockImplementationOnce(
        () => new Promise<void>((resolve) => { resolveLaunch = resolve; })
      );

      const { result } = renderHook(() => useSteam());

      act(() => {
        result.current.launchGame("440");
      });

      // 起動中に別のゲームを起動しようとする
      act(() => {
        result.current.launchGame("730");
      });

      expect(mockInvoke).toHaveBeenCalledTimes(1);

      await act(async () => {
        resolveLaunch();
      });
    });
  });

  // ===== [低優先度] launchSteam のテスト =====

  describe("launchSteam", () => {
    it("成功時: launchingSteam が false に戻る", async () => {
      mockInvoke.mockResolvedValueOnce(undefined);

      const { result } = renderHook(() => useSteam());

      await act(async () => {
        await result.current.launchSteam();
      });

      expect(result.current.launchingSteam).toBe(false);
      expect(result.current.error).toBeNull();
    });

    it("失敗時: error が設定され launchingSteam が false に戻る", async () => {
      mockInvoke.mockRejectedValueOnce(new Error("Steam not installed"));

      const { result } = renderHook(() => useSteam());

      await act(async () => {
        await result.current.launchSteam();
      });

      expect(result.current.launchingSteam).toBe(false);
      expect(result.current.error).not.toBeNull();
      expect(result.current.error).toContain("Steam not installed");
    });

    it("起動中に再度 launchSteam を呼んでも重複実行しない（多重実行防止）", async () => {
      let resolveLaunch!: () => void;
      mockInvoke.mockImplementationOnce(
        () => new Promise<void>((resolve) => { resolveLaunch = resolve; })
      );

      const { result } = renderHook(() => useSteam());

      act(() => {
        result.current.launchSteam();
      });

      // launchingSteam=true の間に再度呼ぶ
      act(() => {
        result.current.launchSteam();
      });

      // invoke は1回しか呼ばれていないはず
      expect(mockInvoke).toHaveBeenCalledTimes(1);

      await act(async () => {
        resolveLaunch();
      });
    });
  });

  // ===== [低優先度] fetchAll 部分失敗テスト =====

  describe("fetchAll 部分失敗", () => {
    it("fetchGames のみ失敗した場合: customIcons は更新され error が設定される", async () => {
      const mockIcons = { "440": "data:image/png;base64,abc" };
      mockInvoke.mockResolvedValueOnce(mockIcons);           // get_custom_icons: 成功
      mockInvoke.mockRejectedValueOnce(new Error("ACF read error")); // get_steam_games: 失敗

      const { result } = renderHook(() => useSteam());

      await act(async () => {
        await result.current.fetchAll();
      });

      // customIcons は更新されている
      expect(result.current.customIcons).toEqual(mockIcons);
      // games は空のまま
      expect(result.current.games).toEqual([]);
      // error は設定されている
      expect(result.current.error).not.toBeNull();
      expect(result.current.error).toContain("ACF read error");
    });
  });

  // ===== [低優先度・バグ検出] fetchGames 直接呼び出し時の error クリア =====

  describe("fetchGames 直接呼び出し時の error クリア", () => {
    it("fetchGames 成功後も前の error がクリアされない（直接呼び出し時）", async () => {
      // 1. まず error を設定する（launchGame 失敗）
      mockInvoke.mockRejectedValueOnce(new Error("Launch failed"));
      const { result } = renderHook(() => useSteam());
      await act(async () => {
        await result.current.launchGame("440");
      });
      expect(result.current.error).not.toBeNull();

      // 2. fetchGames を直接呼んで成功させる
      mockInvoke.mockResolvedValueOnce([
        { appid: "440", name: "Team Fortress 2", header_image_url: "" },
      ]);
      await act(async () => {
        await result.current.fetchGames();
      });

      // バグ: fetchGames 成功後も前の error が残ったまま
      // 修正後はここが null になるべき
      expect(result.current.error).toBeNull();
    });
  });

  // ===== saveCustomIcon のテスト =====

  describe("saveCustomIcon", () => {
    it("成功時: fetchCustomIcons が呼ばれ customIcons が更新される", async () => {
      const updatedIcons = { "440": "data:image/png;base64,newicon" };
      mockInvoke.mockResolvedValueOnce(undefined);     // save_custom_icon_from_path
      mockInvoke.mockResolvedValueOnce(updatedIcons);  // get_custom_icons (fetchCustomIcons)

      const { result } = renderHook(() => useSteam());

      await act(async () => {
        await result.current.saveCustomIcon("440", "C:\\Users\\test\\icon.png");
      });

      expect(result.current.customIcons).toEqual(updatedIcons);
      expect(result.current.error).toBeNull();
    });

    it("失敗時: error が設定される（空ファイル・パス不正等）", async () => {
      mockInvoke.mockRejectedValueOnce(new Error("ファイルが空です"));

      const { result } = renderHook(() => useSteam());

      await act(async () => {
        await result.current.saveCustomIcon("440", "C:\\empty_file.png");
      });

      expect(result.current.error).not.toBeNull();
      expect(result.current.error).toContain("ファイルが空です");
    });
  });
});
