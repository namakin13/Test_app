import { renderHook, act } from "@testing-library/react";
import { vi, describe, it, expect, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { useAudio } from "../useAudio";

const mockInvoke = vi.mocked(invoke);

beforeEach(() => {
  mockInvoke.mockReset();
});

describe("useAudio", () => {
  // ===== fetchDevices のテスト =====

  describe("fetchDevices", () => {
    it("成功時: devices が更新され loading/error がリセットされる", async () => {
      const mockDevices = [
        { id: "dev1", name: "スピーカー", is_default: true },
        { id: "dev2", name: "ヘッドホン", is_default: false },
      ];
      mockInvoke.mockResolvedValueOnce(mockDevices);

      const { result } = renderHook(() => useAudio());

      await act(async () => {
        await result.current.fetchDevices();
      });

      expect(result.current.devices).toEqual(mockDevices);
      expect(result.current.loading).toBe(false);
      expect(result.current.error).toBeNull();
    });

    it("失敗時: error ステートにエラー内容が設定される（UIに伝播）", async () => {
      mockInvoke.mockRejectedValueOnce(new Error("Backend unavailable"));

      const { result } = renderHook(() => useAudio());

      await act(async () => {
        await result.current.fetchDevices();
      });

      // 修正前: error は常に null のままだった
      // 修正後: エラー内容が格納されUIに表示できる
      expect(result.current.error).not.toBeNull();
      expect(result.current.error).toContain("Backend unavailable");
      expect(result.current.loading).toBe(false);
      expect(result.current.devices).toEqual([]);
    });

    it("失敗後に再フェッチが成功した場合、error がクリアされる", async () => {
      // 1回目: 失敗
      mockInvoke.mockRejectedValueOnce(new Error("Temporary error"));
      // 2回目: 成功
      mockInvoke.mockResolvedValueOnce([
        { id: "dev1", name: "スピーカー", is_default: true },
      ]);

      const { result } = renderHook(() => useAudio());

      await act(async () => {
        await result.current.fetchDevices();
      });
      expect(result.current.error).not.toBeNull();

      await act(async () => {
        await result.current.fetchDevices();
      });
      expect(result.current.error).toBeNull();
      expect(result.current.devices).toHaveLength(1);
    });
  });

  // ===== [低優先度] loading 状態のテスト =====

  describe("loading 状態", () => {
    it("初期状態: マウント直後は loading が true（fetchDevices 呼び出し前）", () => {
      const { result } = renderHook(() => useAudio());
      // fetchDevices を呼ぶ前の初期値は true
      expect(result.current.loading).toBe(true);
    });
  });

  // ===== switchDevice のテスト =====

  describe("switchDevice", () => {
    it("成功時: switching が null に戻り fetchDevices が呼ばれる", async () => {
      mockInvoke
        .mockResolvedValueOnce(undefined) // switch_audio_device
        .mockResolvedValueOnce([{ id: "dev1", name: "スピーカー", is_default: true }]); // get_audio_devices

      const { result } = renderHook(() => useAudio());

      await act(async () => {
        await result.current.switchDevice("dev1");
      });

      expect(result.current.switching).toBeNull();
      expect(result.current.error).toBeNull();
      expect(mockInvoke).toHaveBeenCalledWith("switch_audio_device", { id: "dev1" });
    });

    it("失敗時: switching が null に戻り error が設定される", async () => {
      mockInvoke.mockRejectedValueOnce(new Error("Switch failed"));

      const { result } = renderHook(() => useAudio());

      await act(async () => {
        await result.current.switchDevice("dev1");
      });

      expect(result.current.switching).toBeNull();
      expect(result.current.error).not.toBeNull();
      expect(result.current.error).toContain("Switch failed");
    });

    it("失敗時、fetchDevices は呼ばれない（invoke は switch_audio_device の1回のみ）", async () => {
      mockInvoke.mockRejectedValueOnce(new Error("Switch failed"));

      const { result } = renderHook(() => useAudio());

      await act(async () => {
        await result.current.switchDevice("dev1");
      });

      // switch_audio_device の呼び出し1回のみ（fetchDevices=get_audio_devices は呼ばれていない）
      expect(mockInvoke).toHaveBeenCalledTimes(1);
      expect(mockInvoke).toHaveBeenCalledWith("switch_audio_device", { id: "dev1" });
    });

    it("切り替え中に再度 switchDevice を呼んでも重複実行しない（多重実行防止）", async () => {
      let resolveFirst!: () => void;
      mockInvoke.mockImplementationOnce(
        () => new Promise<void>((resolve) => { resolveFirst = resolve; })
      );

      const { result } = renderHook(() => useAudio());

      // 1回目の切り替えを開始（まだ完了しない）
      act(() => {
        result.current.switchDevice("dev1");
      });

      // switching 中に2回目を呼ぶ
      act(() => {
        result.current.switchDevice("dev2");
      });

      // invoke は1回しか呼ばれていないはず
      expect(mockInvoke).toHaveBeenCalledTimes(1);

      // 1回目を完了させる
      await act(async () => {
        resolveFirst();
        // fetchDevices の呼び出し用
        mockInvoke.mockResolvedValueOnce([]);
      });
    });
  });
});
