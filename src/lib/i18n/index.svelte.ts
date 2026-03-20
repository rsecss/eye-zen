import en from './en';
import zhCN, { type TranslationDict, type TranslationKey } from './zh-CN';

export type Locale = 'zh-CN' | 'en';

const locales: Record<Locale, TranslationDict> = {
  'zh-CN': zhCN,
  en,
};

function normalizeLocale(locale: string): Locale | null {
  switch (locale) {
    case 'zh-CN':
      return 'zh-CN';
    case 'en':
    case 'en-US':
      return 'en';
    default:
      return null;
  }
}

function createI18nStore() {
  let locale = $state<Locale>('zh-CN');
  let translations = $state<TranslationDict>(zhCN);

  return {
    get locale(): Locale {
      return locale;
    },

    t(key: TranslationKey): string {
      return translations[key] ?? key;
    },

    setLocale(nextLocale: string) {
      const normalized = normalizeLocale(nextLocale);
      if (!normalized) {
        return;
      }

      locale = normalized;
      translations = locales[normalized];
    },
  };
}

export const i18nStore = createI18nStore();
