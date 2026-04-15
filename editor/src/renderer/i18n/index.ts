import ko from './ko';
import en from './en';
import { useAppStore } from '../stores/appStore';

type Locale = 'ko' | 'en';

const translations: Record<Locale, Record<string, string>> = { ko, en };

/**
 * Get the current locale from the app store.
 * Falls back to 'ko' if the store is not initialized.
 */
function getCurrentLocale(): Locale {
  try {
    return useAppStore.getState().language || 'ko';
  } catch {
    return 'ko';
  }
}

/**
 * Translate a key to the current or specified locale.
 * Falls back to Korean, then returns the key itself if no translation found.
 */
export function t(key: string, locale?: Locale): string {
  const lang = locale || getCurrentLocale();
  return translations[lang]?.[key] || translations['ko']?.[key] || key;
}

/**
 * React hook for translations.
 * Returns a `t` function bound to the current language from the app store.
 */
export function useTranslation() {
  const language = useAppStore((s) => s.language);
  return {
    t: (key: string) => t(key, language),
    locale: language,
  };
}

/**
 * Get all available locales.
 */
export function getAvailableLocales(): { id: Locale; name: string }[] {
  return [
    { id: 'ko', name: '한국어' },
    { id: 'en', name: 'English' },
  ];
}

export type { Locale };
