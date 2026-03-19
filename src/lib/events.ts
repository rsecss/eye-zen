import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { StatePayload } from './bindings/StatePayload';
import type { Config } from './bindings/Config';

export function onStateChanged(cb: (payload: StatePayload) => void): Promise<UnlistenFn> {
  return listen<StatePayload>('state_changed', (e) => cb(e.payload));
}

export function onConfigChanged(cb: (payload: Config) => void): Promise<UnlistenFn> {
  return listen<Config>('config_changed', (e) => cb(e.payload));
}
