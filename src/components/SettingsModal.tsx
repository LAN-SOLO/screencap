import { useEffect, useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { api, Settings, UpdateInfo } from '../api';
import { Dict } from '../i18n';

const APP_VERSION = '0.1.1';

export function SettingsModal({
  settings,
  t,
  onClose,
  onSave,
}: {
  settings: Settings;
  t: Dict;
  onClose: () => void;
  onSave: (s: Settings) => void;
}) {
  const [s, setS] = useState<Settings>({ ...settings });
  const [libPath, setLibPath] = useState('');
  const [updState, setUpdState] = useState<'idle' | 'checking' | 'none' | 'error'>('idle');
  const [update, setUpdate] = useState<UpdateInfo | null>(null);
  const [installing, setInstalling] = useState(false);

  useEffect(() => {
    api.libraryPath().then(setLibPath).catch(() => {});
  }, []);

  const set = <K extends keyof Settings>(key: K, value: Settings[K]) =>
    setS((prev) => ({ ...prev, [key]: value }));

  const pickDir = async () => {
    const picked = await open({ directory: true });
    if (typeof picked === 'string') set('libraryDir', picked);
  };

  const checkUpdates = () => {
    setUpdState('checking');
    setUpdate(null);
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
      .catch(() => setUpdState('error'));
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
            onChange={(e) => set('delayDefault', Math.max(0, Number(e.target.value) || 0))}
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
        <label className="check">
          <input
            type="checkbox"
            checked={s.windowShadow}
            onChange={(e) => set('windowShadow', e.target.checked)}
          />
          {t.windowShadow}
        </label>
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

        <div className="sep" />
        <div className="fieldlabel">{t.updates}</div>
        <div className="updatebox">
          <span>
            {t.version} {APP_VERSION}
          </span>
          <button onClick={checkUpdates} disabled={updState === 'checking'}>
            {updState === 'checking' ? t.checking : t.checkUpdates}
          </button>
          {updState === 'none' && <span>{t.upToDate}</span>}
          {updState === 'error' && <span style={{ color: 'var(--red)' }}>{t.updateError}</span>}
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
                  api.installUpdate().catch(() => setInstalling(false));
                }}
              >
                {t.installUpdate}
              </button>
            </>
          )}
        </div>
        {update?.notes && <div className="note">{update.notes}</div>}

        <div className="note">{t.permissionNote}</div>

        <div className="btnrow">
          <button onClick={onClose}>{t.cancel}</button>
          <button
            className="primary"
            disabled={
              !s.shortcutRegion.trim() || !s.shortcutWindow.trim() || !s.shortcutScreen.trim()
            }
            onClick={() => onSave(s)}
          >
            {t.save}
          </button>
        </div>
      </div>
    </div>
  );
}
