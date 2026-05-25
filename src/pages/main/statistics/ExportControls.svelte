<script lang="ts">
  import { exportStatistics } from '$lib/commands';
  import { save } from '@tauri-apps/plugin-dialog';
  import { i18nStore } from '$lib/i18n/index.svelte';

  let {
    onSuccess,
    onError,
  }: {
    onSuccess?: (message: string) => void;
    onError?: (message: string) => void;
  } = $props();

  let exporting = $state(false);

  function defaultFilename(): string {
    const now = new Date();
    const year = now.getFullYear();
    const month = String(now.getMonth() + 1).padStart(2, '0');
    const day = String(now.getDate()).padStart(2, '0');
    return `eyezen-stat-${year}-${month}-${day}.db`;
  }

  function humanizeError(err: unknown): string {
    if (err instanceof Error) return err.message;
    if (
      typeof err === 'object' &&
      err !== null &&
      'detail' in err &&
      typeof (err as { detail?: unknown }).detail === 'object'
    ) {
      const detail = (err as { detail: Record<string, unknown> }).detail;
      if (typeof detail.reason === 'string') return detail.reason;
      if (typeof detail.message === 'string') return detail.message;
    }
    if (typeof err === 'string') return err;
    return String(err);
  }

  async function handleExport(): Promise<void> {
    if (exporting) return;

    let targetPath: string | null;
    try {
      targetPath = await save({
        title: i18nStore.t('statistics.exportBackup.dialogTitle'),
        defaultPath: defaultFilename(),
        filters: [{ name: 'SQLite Database', extensions: ['db'] }],
      });
    } catch (err) {
      console.error('Failed to open save dialog:', err);
      onError?.(
        i18nStore.t('statistics.exportBackup.toastError').replace('{reason}', humanizeError(err)),
      );
      return;
    }
    if (targetPath === null) return;

    exporting = true;
    try {
      await exportStatistics(targetPath);
      onSuccess?.(
        i18nStore.t('statistics.exportBackup.toastSuccess').replace('{path}', targetPath),
      );
    } catch (err) {
      console.error('Failed to export statistics:', err);
      onError?.(
        i18nStore.t('statistics.exportBackup.toastError').replace('{reason}', humanizeError(err)),
      );
    } finally {
      exporting = false;
    }
  }
</script>

<button class="export-button" onclick={handleExport} disabled={exporting} aria-busy={exporting}>
  {i18nStore.t('statistics.exportBackup.button')}
</button>

<style>
  .export-button {
    padding: 9px 14px;
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--accent-deep);
    font-family: inherit;
    font-weight: 650;
    cursor: pointer;
    transition: var(--transition);
  }

  .export-button:hover:not(:disabled) {
    background: var(--accent-soft);
  }

  .export-button:disabled {
    cursor: wait;
    opacity: 0.55;
  }
</style>
