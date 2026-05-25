import { invoke } from '@tauri-apps/api/core';
import type { StatePayload } from './bindings/StatePayload';
import type { DetectorCapabilities } from './bindings/DetectorCapabilities';
import type { Config } from './bindings/Config';
import type { StatisticsTrendPayload } from './bindings/StatisticsTrendPayload';
import type { CycleOutcomesPayload } from './bindings/CycleOutcomesPayload';
import type { TimerConfig } from './bindings/TimerConfig';
import type { BehaviorConfig } from './bindings/BehaviorConfig';
import type { DisplayConfig } from './bindings/DisplayConfig';
import type { ScheduleConfig } from './bindings/ScheduleConfig';
import type { HotkeyStatus } from './bindings/HotkeyStatus';
import type { HotkeysConfig } from './bindings/HotkeysConfig';
import type { PomodoroConfig } from './bindings/PomodoroConfig';

// Three timeout buckets, picked at each call site:
//   DEFAULT: fast in-memory reads + state mutations
//   IO:      commands that hit SQLite or system capability probes
//   EXPORT:  long-running serialization (VACUUM INTO + filesystem write)
// Keep these aligned with the SHOULD/MUST wording in
// .trellis/spec/architecture/ipc-and-state.md "超时策略".
export const INVOKE_TIMEOUT_DEFAULT_MS = 5_000;
export const INVOKE_TIMEOUT_IO_MS = 10_000;
export const INVOKE_TIMEOUT_EXPORT_MS = 60_000;

function invokeWithTimeout<T>(
  cmd: string,
  args?: Record<string, unknown>,
  timeoutMs: number = INVOKE_TIMEOUT_DEFAULT_MS,
): Promise<T> {
  let timeoutId: ReturnType<typeof setTimeout>;
  return Promise.race([
    invoke<T>(cmd, args),
    new Promise<never>((_, reject) => {
      timeoutId = setTimeout(() => reject(new Error(`Command "${cmd}" timed out`)), timeoutMs);
    }),
  ]).finally(() => clearTimeout(timeoutId));
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

export function getHotkeyStatus(): Promise<HotkeyStatus> {
  return invokeWithTimeout('get_hotkey_status');
}

export function getStatisticsTrends(timezone: string): Promise<StatisticsTrendPayload> {
  return invokeWithTimeout('get_statistics_trends', { timezone }, INVOKE_TIMEOUT_IO_MS);
}

export function getStatisticsCycleOutcomes(timezone: string): Promise<CycleOutcomesPayload> {
  return invokeWithTimeout('statistics_cycle_outcomes', { timezone }, INVOKE_TIMEOUT_IO_MS);
}

export function exportStatistics(targetPath: string): Promise<void> {
  return invokeWithTimeout('export_statistics', { targetPath }, INVOKE_TIMEOUT_EXPORT_MS);
}

export function getDetectorCapabilities(): Promise<DetectorCapabilities> {
  return invokeWithTimeout('get_detector_capabilities', undefined, INVOKE_TIMEOUT_IO_MS);
}

export function updateTimerConfig(config: TimerConfig): Promise<void> {
  return invokeWithTimeout('update_timer_config', { config });
}

export function updateBehaviorConfig(config: BehaviorConfig): Promise<void> {
  return invokeWithTimeout('update_behavior_config', { config });
}

export function updateDisplayConfig(config: DisplayConfig): Promise<void> {
  return invokeWithTimeout('update_display_config', { config });
}

export function updateScheduleConfig(config: ScheduleConfig): Promise<void> {
  return invokeWithTimeout('update_schedule_config', { config });
}

export function updateHotkeysConfig(config: HotkeysConfig): Promise<void> {
  return invokeWithTimeout('update_hotkeys_config', { config }, INVOKE_TIMEOUT_IO_MS);
}

export function updatePomodoroConfig(config: PomodoroConfig): Promise<void> {
  return invokeWithTimeout('update_pomodoro_config', { config });
}
