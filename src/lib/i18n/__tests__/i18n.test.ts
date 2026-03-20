import { beforeEach, describe, expect, it } from 'vitest';

import en from '../en';
import { i18nStore } from '../index.svelte';
import zhCN from '../zh-CN';

describe('i18nStore', () => {
  beforeEach(() => {
    i18nStore.setLocale('zh-CN');
  });

  it('defaults to zh-CN locale', () => {
    expect(i18nStore.locale).toBe('zh-CN');
  });

  it('t returns zh-CN text by default', () => {
    expect(i18nStore.t('tab.settings')).toBe(zhCN['tab.settings']);
  });

  it('setLocale switches to en', () => {
    i18nStore.setLocale('en');
    expect(i18nStore.locale).toBe('en');
    expect(i18nStore.t('tab.settings')).toBe('Settings');
  });

  it('setLocale normalizes en-US to en', () => {
    i18nStore.setLocale('en-US');
    expect(i18nStore.locale).toBe('en');
    expect(i18nStore.t('tab.about')).toBe('About');
  });

  it('setLocale ignores invalid locale', () => {
    i18nStore.setLocale('fr-FR');
    expect(i18nStore.locale).toBe('zh-CN');
  });

  it('t returns key itself for missing key', () => {
    expect(i18nStore.t('nonexistent.key')).toBe('nonexistent.key');
  });
});

describe('translation completeness', () => {
  it('en has all keys that zh-CN has', () => {
    const zhKeys = Object.keys(zhCN).sort();
    const enKeys = Object.keys(en).sort();
    expect(enKeys).toEqual(zhKeys);
  });

  it('no translation value is empty string', () => {
    for (const [key, value] of Object.entries(zhCN)) {
      expect(value, `zh-CN key "${key}" is empty`).not.toBe('');
    }
    for (const [key, value] of Object.entries(en)) {
      expect(value, `en key "${key}" is empty`).not.toBe('');
    }
  });
});
