import { emit, listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { StatePayload } from './bindings/StatePayload';
import type { Config } from './bindings/Config';
import type { HotkeyStatus } from './bindings/HotkeyStatus';
import type { StatPersistenceErrorPayload } from './bindings/StatPersistenceErrorPayload';

export const EVENT_STATE_CHANGED = 'state_changed';
export const EVENT_CONFIG_CHANGED = 'config_changed';
export const EVENT_HOTKEY_STATUS_CHANGED = 'hotkey_status_changed';
export const EVENT_STAT_PERSISTENCE_ERROR = 'stat_persistence_error';
export const EVENT_NAVIGATE_TAB = 'navigate_tab';

export function onStateChanged(callback: (payload: StatePayload) => void): Promise<UnlistenFn> {
  return listen<StatePayload>(EVENT_STATE_CHANGED, (e) => callback(e.payload));
}

export function onConfigChanged(callback: (payload: Config) => void): Promise<UnlistenFn> {
  return listen<Config>(EVENT_CONFIG_CHANGED, (e) => callback(e.payload));
}

export function onHotkeyStatusChanged(
  callback: (payload: HotkeyStatus) => void,
): Promise<UnlistenFn> {
  return listen<HotkeyStatus>(EVENT_HOTKEY_STATUS_CHANGED, (e) => callback(e.payload));
}

export function onStatPersistenceError(
  callback: (payload: StatPersistenceErrorPayload) => void,
): Promise<UnlistenFn> {
  return listen<StatPersistenceErrorPayload>(EVENT_STAT_PERSISTENCE_ERROR, (e) =>
    callback(e.payload),
  );
}

export function onNavigateTab(callback: (tab: string) => void): Promise<UnlistenFn> {
  return listen<string>(EVENT_NAVIGATE_TAB, (e) => callback(e.payload));
}

export function emitNavigateTab(tab: string): Promise<void> {
  return emit(EVENT_NAVIGATE_TAB, tab);
}
