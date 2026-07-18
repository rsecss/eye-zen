import { i18nStore } from '$lib/i18n/index.svelte';

function isAppErrorLike(err: unknown): err is {
  kind?: unknown;
  detail?: { reason?: unknown; message?: unknown };
} {
  return typeof err === 'object' && err !== null && 'detail' in err;
}

export function errorMessage(err: unknown): string {
  if (err instanceof Error) return err.message;
  if (isAppErrorLike(err)) {
    if (err.kind === 'ConfigConflict') {
      return i18nStore.t('settings.config.error.conflict');
    }
    const detail = err.detail;
    if (typeof detail?.reason === 'string') return detail.reason;
    if (typeof detail?.message === 'string') return detail.message;
  }
  return i18nStore.t('settings.config.error.generic');
}
