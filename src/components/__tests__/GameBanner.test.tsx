import { render, screen, fireEvent, waitFor, act } from "@testing-library/react";
import { vi, describe, it, expect, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { GameBanner } from "../GameBanner";

const mockInvoke = vi.mocked(invoke);

const mockGame = {
  name: "Team Fortress 2",
  appid: "440",
  header_image_url:
    "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/440/library_600x900.jpg",
};

const cdnPortrait440 =
  "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/440/library_600x900.jpg";
const cdnHeader440 =
  "https://cdn.akamai.steamstatic.com/steam/apps/440/header.jpg";

beforeEach(() => {
  mockInvoke.mockReset();
  mockInvoke.mockResolvedValue(null); // デフォルト: ローカルキャッシュなし
});

describe("GameBanner フォールバックチェーン", () => {
  // ===== 初期表示 =====

  describe("初期表示", () => {
    it("customIcon が提供された場合、最初のsrcに使われる", () => {
      const customIcon = "data:image/png;base64,customicon123";
      render(<GameBanner game={mockGame} customIcon={customIcon} />);

      const img = screen.getByRole("img");
      expect(img.getAttribute("src")).toBe(customIcon);
    });

    it("customIconなし・localCacheなしの場合、cdnPortrait が最初のsrc", () => {
      render(<GameBanner game={mockGame} />);

      const img = screen.getByRole("img");
      expect(img.getAttribute("src")).toBe(cdnPortrait440);
    });

    it("localCacheImg が取得できた場合、cdnPortrait より優先される", async () => {
      const localCache = "data:image/jpeg;base64,localcacheimage";
      mockInvoke.mockResolvedValue(localCache);

      render(<GameBanner game={mockGame} />);

      // useEffect による非同期フェッチが完了するまで待機
      await waitFor(() => {
        const img = screen.getByRole("img");
        expect(img.getAttribute("src")).toBe(localCache);
      });
    });

    it("customIconとlocalCacheが両方ある場合、customIconが最優先", async () => {
      const customIcon = "data:image/png;base64,customicon";
      const localCache = "data:image/jpeg;base64,localcache";
      mockInvoke.mockResolvedValue(localCache);

      render(<GameBanner game={mockGame} customIcon={customIcon} />);

      // localCacheが届いた後もcustomIconが最初のsrcであること
      await waitFor(() => {
        const img = screen.getByRole("img");
        expect(img.getAttribute("src")).toBe(customIcon);
      });
    });
  });

  // ===== エラー時の遷移 =====

  describe("エラー時のフォールバック遷移", () => {
    it("cdnPortrait 失敗時に cdnHeader に進む", async () => {
      render(<GameBanner game={mockGame} />);
      const img = screen.getByRole("img");

      expect(img.getAttribute("src")).toBe(cdnPortrait440);

      fireEvent.error(img);
      await waitFor(() => {
        expect(img.getAttribute("src")).toBe(cdnHeader440);
      });
    });

    it("全フォールバック失敗時、img が display:none になる", async () => {
      render(<GameBanner game={mockGame} />);
      const img = screen.getByRole("img");

      // cdnPortrait → cdnHeader
      fireEvent.error(img);
      await waitFor(() => expect(img.getAttribute("src")).toBe(cdnHeader440));

      // cdnHeader → 全フォールバック消費
      fireEvent.error(img);
      await waitFor(() => {
        expect(img.style.display).toBe("none");
      });
    });

    it("全フォールバック失敗時、プレースホルダー (.game-banner-fallback) の 'hidden' クラスが外れる", async () => {
      const { container } = render(<GameBanner game={mockGame} />);
      const img = screen.getByRole("img");

      fireEvent.error(img);
      await waitFor(() => expect(img.getAttribute("src")).toBe(cdnHeader440));
      fireEvent.error(img);

      await waitFor(() => {
        const fallbackDiv = container.querySelector(".game-banner-fallback");
        expect(fallbackDiv?.className).not.toContain("hidden");
      });
    });

    it("game.appid 変更で imgIndex がリセットされ、新しいゲームの cdnPortrait が表示される", async () => {
      const { rerender } = render(<GameBanner game={mockGame} />);
      const img = screen.getByRole("img");

      // エラーを発生させて cdnHeader まで進める
      fireEvent.error(img);
      await waitFor(() => expect(img.getAttribute("src")).toBe(cdnHeader440));

      // 別のゲームに変更
      const newGame = {
        name: "Counter-Strike 2",
        appid: "730",
        header_image_url: "",
      };
      const cdnPortrait730 =
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/730/library_600x900.jpg";

      rerender(<GameBanner game={newGame} />);

      // imgIndex がリセットされて新ゲームの cdnPortrait に戻るはず
      await waitFor(() => {
        expect(img.getAttribute("src")).toBe(cdnPortrait730);
      });
    });

    it("customIcon 失敗後、localCache → cdnPortrait → cdnHeader の順でフォールバックする", async () => {
      const customIcon = "data:image/png;base64,broken_custom_icon";
      const localCache = "data:image/jpeg;base64,localcache";

      // localCache の到着タイミングを明示的に制御する
      let resolveLocalCache!: (value: string | null) => void;
      mockInvoke.mockImplementation(
        () => new Promise((resolve) => { resolveLocalCache = resolve; })
      );

      render(<GameBanner game={mockGame} customIcon={customIcon} />);
      const img = screen.getByRole("img");

      // customIcon が最初 (localCache 未到着)
      expect(img.getAttribute("src")).toBe(customIcon);

      // localCache を明示的に到着させる
      // → setLocalCacheImg が呼ばれ、useEffect が imgIndex を 0 にリセット
      // → fallbacks = [customIcon, localCache, cdnPortrait, cdnHeader], imgIndex = 0
      await act(async () => {
        resolveLocalCache(localCache);
      });

      // imgIndex リセット後も index 0 = customIcon のまま
      expect(img.getAttribute("src")).toBe(customIcon);

      // customIcon 失敗 → localCache (index 1)
      fireEvent.error(img);
      await waitFor(() => expect(img.getAttribute("src")).toBe(localCache));

      // localCache 失敗 → cdnPortrait (index 2)
      fireEvent.error(img);
      await waitFor(() => expect(img.getAttribute("src")).toBe(cdnPortrait440));

      // cdnPortrait 失敗 → cdnHeader (index 3)
      fireEvent.error(img);
      await waitFor(() => expect(img.getAttribute("src")).toBe(cdnHeader440));
    });
  });

  // ===== バグ検出テスト =====

  describe("[バグ検出] localCacheImg 遅延到着による imgIndex のずれ", () => {
    it("cdnPortrait 失敗後に localCache が届いても、既失敗の cdnPortrait を再試行しない", async () => {
      // localCache の解決を手動でコントロールする
      let resolveLocalCache!: (value: string | null) => void;
      mockInvoke.mockImplementation(
        () =>
          new Promise((resolve) => {
            resolveLocalCache = resolve;
          })
      );

      render(<GameBanner game={mockGame} />);
      const img = screen.getByRole("img");

      // localCache 未到着の状態で cdnPortrait 表示中
      expect(img.getAttribute("src")).toBe(cdnPortrait440);

      // cdnPortrait 失敗 → imgIndex = 1 → cdnHeader
      fireEvent.error(img);
      await waitFor(() => expect(img.getAttribute("src")).toBe(cdnHeader440));

      // この状態で localCache が遅れて到着
      // → fallbacks が [localCache, cdnPortrait, cdnHeader] に変わる
      // → imgIndex = 1 が cdnPortrait を指し直す（バグ）
      await act(async () => {
        resolveLocalCache("data:image/jpeg;base64,latearrival");
      });

      // 期待: 既に失敗した cdnPortrait を再試行しない
      // バグがある場合: src が cdnPortrait (library_600x900.jpg) に戻る
      const currentSrc = img.getAttribute("src");
      expect(currentSrc).not.toBe(cdnPortrait440); // ← バグがあるとここで FAIL
    });
  });
});
