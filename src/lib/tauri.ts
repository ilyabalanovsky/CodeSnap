import { invoke } from '@tauri-apps/api/core';

function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

export async function getClipboardText(): Promise<string> {
  try {
    return await invoke<string>('get_clipboard_text');
  } catch {
    return '';
  }
}

export async function copySelectedTextThenOpen(): Promise<string> {
  try {
    return await invoke<string>('copy_selected_text_then_open');
  } catch {
    return '';
  }
}

export type SaveSnapshotResult = {
  saved: boolean;
  path: string | null;
};

export async function saveSnapshotPng(dataUrl: string, suggestedFileName: string): Promise<SaveSnapshotResult> {
  return invoke<SaveSnapshotResult>('save_snapshot_png', { dataUrl, suggestedFileName });
}

export async function copySnapshotPng(dataUrl: string): Promise<boolean> {
  try {
    return await invoke<boolean>('copy_snapshot_png', { dataUrl });
  } catch {
    return false;
  }
}

export async function hideToTray(): Promise<void> {
  try {
    await invoke('hide_to_tray');
  } catch {
    // Browser preview mode does not expose Tauri commands.
  }
}

export type AppSettings = {
  captureHotkey: string;
  launchAtLogin: boolean;
  startInTray: boolean;
  disableAnimations: boolean;
};

export const defaultAppSettings: AppSettings = {
  captureHotkey: 'Ctrl+Shift+S',
  launchAtLogin: false,
  startInTray: false,
  disableAnimations: false,
};

export async function getAppSettings(): Promise<AppSettings> {
  try {
    return await invoke<AppSettings>('get_app_settings');
  } catch (error) {
    if (isTauriRuntime()) {
      throw error;
    }

    return defaultAppSettings;
  }
}

export async function setAppSettings(settings: AppSettings): Promise<AppSettings> {
  try {
    return await invoke<AppSettings>('set_app_settings', { settings });
  } catch (error) {
    if (isTauriRuntime()) {
      throw error;
    }

    return settings;
  }
}
