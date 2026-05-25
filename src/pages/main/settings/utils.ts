import { i18nStore } from '$lib/i18n/index.svelte';

function isAppErrorLike(err: unknown): err is {
  detail?: { reason?: unknown; message?: unknown };
} {
  return typeof err === 'object' && err !== null && 'detail' in err;
}

export function errorMessage(err: unknown): string {
  if (err instanceof Error) return err.message;
  if (isAppErrorLike(err)) {
    const detail = err.detail;
    if (typeof detail?.reason === 'string') return detail.reason;
    if (typeof detail?.message === 'string') return detail.message;
  }
  return i18nStore.t('settings.hotkeys.error.generic');
}
