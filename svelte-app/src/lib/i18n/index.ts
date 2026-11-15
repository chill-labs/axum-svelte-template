import type { ErrorCode } from '#lib/api/types.generated.ts';
import { en } from './en';
import type { Dictionary, Locale } from './types';
import { vi } from './vi';

export type { Dictionary, Locale };

const DICTIONARIES: Record<Locale, Dictionary> = { vi, en };

export function normalizeLocale(raw?: string | null): Locale {
  return raw?.toLowerCase().startsWith('en') ? 'en' : 'vi';
}

export function getDictionary(locale: Locale): Dictionary {
  return DICTIONARIES[locale] ?? vi;
}

export function translateError(locale: Locale, code?: string | null): string {
  const dict = getDictionary(locale);
  if (!code) return dict.errors.UNKNOWN_ERROR;
  if (code in dict.errors) {
    return dict.errors[code as keyof Dictionary['errors']];
  }
  return code as ErrorCode extends never ? string : string;
}
