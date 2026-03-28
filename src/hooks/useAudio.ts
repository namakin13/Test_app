import { useState } from "react";
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

  const fetchDevices = async () => {
    setLoading(true);
    try {
      const result: AudioDevice[] = await invoke("get_audio_devices");
      setDevices(result);
    } catch (error) {
      console.error("Failed to fetch devices:", error);
    } finally {
      setLoading(false);
    }
  };

  const switchDevice = async (id: string) => {
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

  return { devices, loading, switching, fetchDevices, switchDevice };
};
