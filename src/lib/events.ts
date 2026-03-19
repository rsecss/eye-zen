import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { StatePayload } from './bindings/StatePayload';
import type { Config } from './bindings/Config';

export function onStateChanged(callback: (payload: StatePayload) => void): Promise<UnlistenFn> {
  return listen<StatePayload>('state_changed', (e) => callback(e.payload));
}

export function onConfigChanged(callback: (payload: Config) => void): Promise<UnlistenFn> {
  return listen<Config>('config_changed', (e) => callback(e.payload));
}
