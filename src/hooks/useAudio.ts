import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface AudioDevice {
  id: string;
  name: string;
  is_default: boolean;
}

export const useAudio = () => {
  const [devices, setDevices] = useState<AudioDevice[]>([]);
  const [loading, setLoading] = useState(true);
  const [switching, setSwitching] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [volume, setVolumeState] = useState<number>(0);

  const fetchDevices = async () => {
    setLoading(true);
    setError(null);
    try {
      const result: AudioDevice[] = await invoke("get_audio_devices");
      setDevices(result);
    } catch (err) {
      console.error("Failed to fetch devices:", err);
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const switchDevice = async (id: string) => {
    if (switching) return;
    setSwitching(id);
    setError(null);
    try {
      await invoke("switch_audio_device", { id });
      await fetchDevices();
    } catch (err) {
      console.error("Failed to switch device:", err);
      setError(String(err));
    } finally {
      setSwitching(null);
    }
  };

  const fetchVolume = useCallback(async (id: string) => {
    try {
      const level: number = await invoke("get_device_volume", { id });
      setVolumeState(Math.round(level * 100));
    } catch (err) {
      console.error("Failed to fetch volume:", err);
    }
  }, []);

  // スライダー操作中の即時反映用。値はパーセント(0-100)。
  const setVolume = useCallback(async (id: string, percent: number) => {
    const clamped = Math.max(0, Math.min(100, Math.round(percent)));
    setVolumeState(clamped);
    try {
      await invoke("set_device_volume", { id, level: clamped / 100 });
    } catch (err) {
      console.error("Failed to set volume:", err);
    }
  }, []);

  // デフォルトデバイスが変わったら音量を取り直す
  const defaultId = devices.find((d) => d.is_default)?.id ?? null;
  useEffect(() => {
    if (defaultId) fetchVolume(defaultId);
  }, [defaultId, fetchVolume]);

  return {
    devices, loading, switching, error, volume,
    fetchDevices, switchDevice, fetchVolume, setVolume,
  };
};
