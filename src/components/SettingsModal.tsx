import { useEffect, useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { getVersion } from '@tauri-apps/api/app';
import { api, Settings, UpdateInfo } from '../api';
import { Dict } from '../i18n';
import { IS_WINDOWS } from '../platform';

// Fallback, falls getVersion() (tauri.conf.json) nicht verfügbar ist
const APP_VERSION = '1.0.0';

export function SettingsModal({
  settings,
  t,
  onClose,
  onSave,
}: {
  settings: Settings;
  t: Dict;
  onClose: () => void;
  onSave: (s: Settings) => Promise<void>;
}) {
  const [s, setS] = useState<Settings>({ ...settings });
  const [libPath, setLibPath] = useState('');
  const [updState, setUpdState] = useState<'idle' | 'checking' | 'none' | 'error'>('idle');
  const [updError, setUpdError] = useState<string | null>(null);
  const [update, setUpdate] = useState<UpdateInfo | null>(null);
  const [installing, setInstalling] = useState(false);
  const [version, setVersion] = useState(APP_VERSION);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [shortcutError, setShortcutError] = useState<string | null>(null);

  useEffect(() => {
    api.libraryPath().then(setLibPath).catch(() => {});
    getVersion().then(setVersion).catch(() => {});
    api
      .appStatus()
      .then((st) => setShortcutError(st.shortcutError))
      .catch(() => {});
  }, []);

  // Esc schließt den Dialog (ohne Speichern) — nicht während gespeichert wird
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && !saving) {
        e.preventDefault();
        onClose();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [onClose, saving]);

  const doSave = () => {
    setSaving(true);
    setSaveError(null);
    onSave(s)
      .catch((e) => setSaveError(String(e)))
      .finally(() => setSaving(false));
  };

  const set = <K extends keyof Settings>(key: K, value: Settings[K]) =>
    setS((prev) => ({ ...prev, [key]: value }));

  const pickDir = async () => {
    try {
      const picked = await open({ directory: true });
      if (typeof picked === 'string') set('libraryDir', picked);
    } catch (e) {
      setSaveError(String(e));
    }
  };

  const checkUpdates = () => {
    setUpdState('checking');
    setUpdate(null);
    setUpdError(null);
    api
      .checkUpdate()
      .then((u) => {
        if (u) {
          setUpdate(u);
          setUpdState('idle');
        } else {
          setUpdState('none');
        }
      })
      .catch((e) => {
        setUpdError(String(e));
        setUpdState('error');
      });
  };

  return (
    <div className="overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <h2>
          <span className="brand">
            <span className="name">screencap</span>
            <span className="dot">.</span>
          </span>{' '}
          — {t.settings}
        </h2>

        <label className="field">
          <span>{t.language}</span>
          <select value={s.language} onChange={(e) => set('language', e.target.value as 'de' | 'en')}>
            <option value="de">Deutsch</option>
            <option value="en">English</option>
          </select>
        </label>

        <label className="field">
          <span>{t.themeLabel}</span>
          <select value={s.theme} onChange={(e) => set('theme', e.target.value as Settings['theme'])}>
            <option value="dark">{t.themeDark}</option>
            <option value="light">{t.themeLight}</option>
          </select>
        </label>

        <label className="field">
          <span>{t.libraryDir}</span>
          <div className="row2">
            <input
              type="text"
              value={s.libraryDir}
              placeholder={libPath || t.libraryDirHint}
              onChange={(e) => set('libraryDir', e.target.value)}
            />
            <button onClick={pickDir}>{t.choose}</button>
          </div>
        </label>

        <label className="field">
          <span>{t.format}</span>
          <select value={s.format} onChange={(e) => set('format', e.target.value as 'png' | 'jpg')}>
            <option value="png">PNG</option>
            <option value="jpg">JPG</option>
          </select>
        </label>

        <label className="field">
          <span>{t.delayDefault}</span>
          <input
            type="number"
            min={0}
            max={60}
            value={s.delayDefault}
            onChange={(e) =>
              set('delayDefault', Math.min(60, Math.max(0, Math.round(Number(e.target.value)) || 0)))
            }
          />
        </label>

        <label className="check">
          <input
            type="checkbox"
            checked={s.copyAfterCapture}
            onChange={(e) => set('copyAfterCapture', e.target.checked)}
          />
          {t.copyAfterCapture}
        </label>
        <label className="check">
          <input
            type="checkbox"
            checked={s.openEditorAfter}
            onChange={(e) => set('openEditorAfter', e.target.checked)}
          />
          {t.openEditorAfter}
        </label>
        {!IS_WINDOWS && (
          <label className="check">
            <input
              type="checkbox"
              checked={s.windowShadow}
              onChange={(e) => set('windowShadow', e.target.checked)}
            />
            {t.windowShadow}
          </label>
        )}
        <label className="check">
          <input
            type="checkbox"
            checked={s.autostart}
            onChange={(e) => set('autostart', e.target.checked)}
          />
          {t.autostart}
        </label>

        <div className="sep" />
        <div className="fieldlabel">{t.shortcuts}</div>
        <label className="field">
          <span>{t.shortcutRegion}</span>
          <input
            type="text"
            value={s.shortcutRegion}
            onChange={(e) => set('shortcutRegion', e.target.value)}
          />
        </label>
        <label className="field">
          <span>{t.shortcutWindow}</span>
          <input
            type="text"
            value={s.shortcutWindow}
            onChange={(e) => set('shortcutWindow', e.target.value)}
          />
        </label>
        <label className="field">
          <span>{t.shortcutScreen}</span>
          <input
            type="text"
            value={s.shortcutScreen}
            onChange={(e) => set('shortcutScreen', e.target.value)}
          />
        </label>
        <div className="note">{t.shortcutHint}</div>
        {shortcutError && (
          <div className="note" style={{ color: 'var(--red)' }}>
            {t.shortcutErrorNote} {shortcutError}
          </div>
        )}

        <div className="sep" />
        <div className="fieldlabel">{t.updates}</div>
        <div className="updatebox">
          <span>
            {t.version} {version}
          </span>
          <button onClick={checkUpdates} disabled={updState === 'checking'}>
            {updState === 'checking' ? t.checking : t.checkUpdates}
          </button>
          {updState === 'none' && <span>{t.upToDate}</span>}
          {updState === 'error' && !updError && <span style={{ color: 'var(--red)' }}>{t.updateError}</span>}
          {update && (
            <>
              <span>
                {t.updateAvailable} <strong>{update.version}</strong>
              </span>
              <button
                className="primary"
                disabled={installing}
                onClick={() => {
                  setInstalling(true);
                  setUpdError(null);
                  api.installUpdate().catch((e) => {
                    setInstalling(false);
                    setUpdError(String(e)); // schon mit Präfix aus dem Backend
                  });
                }}
              >
                {installing ? t.updateInstalling : t.installUpdate}
              </button>
            </>
          )}
        </div>
        {updError && (
          <div className="note" style={{ color: 'var(--red)', overflowWrap: 'anywhere' }}>
            {updError}
          </div>
        )}
        {update?.notes && <div className="note">{update.notes}</div>}

        <div className="note">{IS_WINDOWS ? t.permissionNoteWin : t.permissionNote}</div>
        {saveError && (
          <div className="note" style={{ color: 'var(--red)', overflowWrap: 'anywhere' }}>
            {saveError}
          </div>
        )}

        <div className="btnrow">
          <button onClick={onClose}>{t.cancel}</button>
          <button
            className="primary"
            disabled={
              saving || !s.shortcutRegion.trim() || !s.shortcutWindow.trim() || !s.shortcutScreen.trim()
            }
            onClick={doSave}
          >
            {t.save}
          </button>
        </div>
      </div>
    </div>
  );
}
