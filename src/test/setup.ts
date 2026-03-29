import { vi } from "vitest";

// @tauri-apps/api/core の invoke をモック化
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));
