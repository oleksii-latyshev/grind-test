import { useEffect } from "react";
import { IntlProvider, useIntl } from "react-intl";
import type { IntlShape } from "react-intl";

import { usePreference } from "@/lib/prefs";

import { en } from "./messages/en";
import type { MessageId } from "./messages/en";
import { ru } from "./messages/ru";
import { uk } from "./messages/uk";

export type Locale = "uk" | "ru" | "en";

export const LOCALES: { value: Locale; label: string }[] = [
  { value: "uk", label: "Українська" },
  { value: "ru", label: "Русский" },
  { value: "en", label: "English" },
];

const CATALOGUES: Record<Locale, Record<MessageId, string>> = { uk, ru, en };

/**
 * Interface language.
 *
 * This moves the *chrome* only. Study notes, questions, explanations and hints stay
 * Ukrainian whatever this is set to, because they are generated against a Ukrainian exam —
 * the prompts demand Ukrainian output and are not affected by this setting. Someone
 * revising in Ukraine may still want an English UI; nobody wants an English answer to a
 * Ukrainian exam question.
 */
export function useLocale() {
  return usePreference<Locale>(
    "grind:locale",
    "uk",
    (value): value is Locale => value === "uk" || value === "ru" || value === "en",
  );
}

export function AppIntlProvider({
  locale,
  children,
}: {
  locale: Locale;
  children: React.ReactNode;
}) {
  // Assistive technology and the browser's own text handling read this, not the provider.
  useEffect(() => {
    document.documentElement.lang = locale;
  }, [locale]);

  return (
    <IntlProvider
      locale={locale}
      defaultLocale="uk"
      messages={CATALOGUES[locale]}
      // Every catalogue is complete by construction — `Record<MessageId, string>` — so a
      // miss is a bug in this app, not a translation gap worth logging on every render.
      onError={() => {}}
    >
      {children}
    </IntlProvider>
  );
}

/** Typed `formatMessage`, so a typo in an id fails to compile rather than at runtime. */
export function useT() {
  const intl = useIntl();
  return (id: MessageId, values?: Record<string, string | number>) =>
    intl.formatMessage({ id }, values);
}

export type { MessageId };
export type Translate = (id: MessageId, values?: Record<string, string | number>) => string;

/** The same thing outside a component, for helpers that already receive an `IntlShape`. */
export function translator(intl: IntlShape): Translate {
  return (id, values) => intl.formatMessage({ id }, values);
}
