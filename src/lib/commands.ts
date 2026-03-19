import { invoke } from '@tauri-apps/api/core';
import type { StatePayload } from './bindings/StatePayload';
import type { Config } from './bindings/Config';

const TIMEOUT_MS = 5000;

function invokeWithTimeout<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return Promise.race([
    invoke<T>(cmd, args),
    new Promise<never>((_, reject) =>
      setTimeout(() => reject(new Error(`Command "${cmd}" timed out`)), TIMEOUT_MS),
    ),
  ]);
}

export function getStateSnapshot(): Promise<StatePayload> {
  return invokeWithTimeout('get_state_snapshot');
}

export function startRest(): Promise<void> {
  return invokeWithTimeout('start_rest');
}

export function skipRest(): Promise<void> {
  return invokeWithTimeout('skip_rest');
}

export function pauseTimer(): Promise<void> {
  return invokeWithTimeout('pause_timer');
}

export function resumeTimer(): Promise<void> {
  return invokeWithTimeout('resume_timer');
}

export function getConfig(): Promise<Config> {
  return invokeWithTimeout('get_config');
}
