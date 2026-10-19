import { confirm } from '@tauri-apps/plugin-dialog';

/**
 * Native Rückfrage (tauri-plugin-dialog). Niemals window.confirm: in der
 * Tauri-WebView ist das ein asynchroner Stub, `if (window.confirm(..))` wäre
 * immer wahr. Die Knöpfe tragen die Aktion in der UI-Sprache („Löschen“ /
 * „Abbrechen“) statt der englischen Standardtexte; Esc/Schließen = Abbrechen.
 * Schlägt der Dialog selbst fehl, gilt das als „Nein“ — eine destruktive
 * Aktion läuft nie ohne ausdrückliche Bestätigung.
 */
export async function askConfirm(
  message: string,
  okLabel: string,
  cancelLabel: string,
  kind: 'warning' | 'info' = 'warning'
): Promise<boolean> {
  try {
    return await confirm(message, { title: 'screencap', kind, okLabel, cancelLabel });
  } catch {
    return false;
  }
}
