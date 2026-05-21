import { emit, listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { StatePayload } from './bindings/StatePayload';
import type { Config } from './bindings/Config';
import type { HotkeyStatus } from './bindings/HotkeyStatus';

export function onStateChanged(callback: (payload: StatePayload) => void): Promise<UnlistenFn> {
  return listen<StatePayload>('state_changed', (e) => callback(e.payload));
}

export function onConfigChanged(callback: (payload: Config) => void): Promise<UnlistenFn> {
  return listen<Config>('config_changed', (e) => callback(e.payload));
}

export function onHotkeyStatusChanged(
  callback: (payload: HotkeyStatus) => void,
): Promise<UnlistenFn> {
  return listen<HotkeyStatus>('hotkey_status_changed', (e) => callback(e.payload));
}

export function onNavigateTab(callback: (tab: string) => void): Promise<UnlistenFn> {
  return listen<string>('navigate_tab', (e) => callback(e.payload));
}

export function emitNavigateTab(tab: string): Promise<void> {
  return emit('navigate_tab', tab);
}
