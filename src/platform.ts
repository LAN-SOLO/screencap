// Plattform-Erkennung für Beschriftungen (Finder vs. Explorer, ⌘ vs. Ctrl)
// und macOS-only-Optionen. Die WebView meldet das OS im User-Agent.
export const IS_WINDOWS = typeof navigator !== 'undefined' && /windows/i.test(navigator.userAgent);
export const IS_MAC = typeof navigator !== 'undefined' && /mac os x|macintosh/i.test(navigator.userAgent);

/** System-Sprache der WebView → 'de' | 'en' */
export function systemLang(): 'de' | 'en' {
  try {
    return (navigator.language || '').toLowerCase().startsWith('de') ? 'de' : 'en';
  } catch {
    return 'en';
  }
}
