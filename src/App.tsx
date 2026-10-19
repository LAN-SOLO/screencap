import { useCallback, useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import { api, Filter, Settings, Shot, UpdateInfo } from './api';
import { dicts, Lang } from './i18n';
import { IS_WINDOWS, systemLang } from './platform';
import { askConfirm } from './dialog';
import { Editor } from './components/Editor';
import { SettingsModal } from './components/SettingsModal';
import { Help } from './components/Help';
import {
  IconCopy,
  IconFolder,
  IconGear,
  IconImport,
  IconRegion,
  IconScreen,
  IconStar,
  IconTrash,
  IconWindow,
} from './icons';

const thumbCache = new Map<string, string>();

// Cache-Schlüssel enthält Datei, Maße und Größe: nach „Speichern“ (Überschreiben)
// ändert sich mindestens eins davon, das alte Thumbnail fliegt damit raus.
const thumbKey = (s: Shot) => `${s.id}|${s.file}|${s.width}x${s.height}|${s.sizeBytes}`;

function Thumb({ shot }: { shot: Shot }) {
  const key = thumbKey(shot);
  const [src, setSrc] = useState<string | null>(thumbCache.get(key) ?? null);
  useEffect(() => {
    let alive = true;
    if (thumbCache.has(key)) {
      setSrc(thumbCache.get(key)!);
      return;
    }
    api
      .getThumb(shot.id)
      .then((data) => {
        thumbCache.set(key, data);
        if (alive) setSrc(data);
      })
      .catch(() => {});
    return () => {
      alive = false;
    };
  }, [key, shot.id]);
  return src ? <img src={src} alt="" /> : null;
}

function fmtSize(b: number) {
  return b > 1024 * 1024 ? `${(b / 1024 / 1024).toFixed(1)} MB` : `${Math.round(b / 1024)} KB`;
}

function fmtDate(iso: string, lang: Lang) {
  return new Date(iso).toLocaleString(lang === 'de' ? 'de-DE' : 'en-US', {
    day: '2-digit',
    month: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  });
}

export default function App() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [shots, setShots] = useState<Shot[]>([]);
  const [query, setQuery] = useState('');
  const [filter, setFilter] = useState<Filter>('all');
  const [delay, setDelay] = useState(0);
  const [editorShot, setEditorShot] = useState<Shot | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [toast, setToast] = useState<{ msg: string; error: boolean } | null>(null);
  const [updateAvail, setUpdateAvail] = useState<UpdateInfo | null>(null);
  const [installing, setInstalling] = useState(false);
  const toastTimer = useRef<number | undefined>(undefined);
  const shotsRef = useRef<Shot[]>([]);
  shotsRef.current = shots;
  // ungespeicherte Editor-Änderungen (vom Editor gemeldet)
  const dirtyRef = useRef(false);
  const tRef = useRef(dicts.de);

  const lang: Lang = settings?.language ?? 'de';
  const t = dicts[lang];
  tRef.current = t;
  const revealLabel = IS_WINDOWS ? t.revealWin : t.reveal;

  // Bestätigungen verschwinden nach 1,5 s; Fehler bleiben 8 s stehen (lang
  // genug zum Lesen) und lassen sich per Klick schließen
  const showToast = useCallback((msg: string) => {
    setToast({ msg, error: false });
    window.clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToast(null), 1500);
  }, []);
  const showError = useCallback((e: unknown) => {
    setToast({ msg: String(e), error: true });
    window.clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToast(null), 8000);
  }, []);

  const refresh = useCallback(() => {
    api.listShots(query, filter).then(setShots).catch(showError);
  }, [query, filter, showError]);

  useEffect(() => {
    Promise.all([api.getSettings(), api.appStatus().catch(() => null)]).then(([s, status]) => {
      // erster Start: Sprache des Systems übernehmen (GUI-Apps sehen unter
      // macOS/Windows kein $LANG — die WebView kennt die Systemsprache)
      if (status?.firstRun && s.language !== systemLang()) {
        const adopted = { ...s, language: systemLang() };
        api.setSettings(adopted).catch(() => {});
        s = adopted;
      }
      setSettings(s);
      setDelay(s.delayDefault);
    });
    // stiller Update-Check beim Start — installiert wird nur nach Klick
    api.checkUpdate().then(setUpdateAvail).catch(() => {});
  }, []);

  useEffect(refresh, [refresh]);

  // Darstellung: data-theme auf <html>, styles.css schaltet die Palette um.
  // localStorage spiegelt den Wert, damit main.tsx ihn vor dem ersten Render setzen kann.
  useEffect(() => {
    if (!settings) return;
    const theme = settings.theme === 'light' ? 'light' : 'dark';
    document.documentElement.dataset.theme = theme;
    try {
      localStorage.setItem('theme', theme);
    } catch {
      /* Storage nicht verfügbar — kein Problem, Settings sind die Quelle */
    }
  }, [settings]);

  useEffect(() => {
    const subs = [
      listen('library-changed', refresh),
      listen<string>('open-editor', (e) => {
        // the library event may arrive after this one — fetch fresh
        api.listShots('', 'all').then((all) => {
          const s = all.find((x) => x.id === e.payload);
          if (s) openEditor(s);
        });
      }),
      // Fehler bei Kürzel-/Tray-Aufnahmen (kein invoke-Aufrufer, der sie anzeigt)
      listen<string>('capture-error', (e) => showError(e.payload)),
    ];
    return () => {
      subs.forEach((p) => p.then((un) => un()));
    };
  }, [refresh]);

  // keep editor shot in sync with library updates
  useEffect(() => {
    if (!editorShot) return;
    const fresh = shots.find((s) => s.id === editorShot.id);
    if (fresh && JSON.stringify(fresh) !== JSON.stringify(editorShot)) {
      setEditorShot(fresh);
    }
  }, [shots, editorShot]);

  // Editor verlassen/wechseln — bei ungespeicherten Änderungen erst fragen
  async function openEditor(next: Shot | null) {
    if (dirtyRef.current) {
      const tt = tRef.current;
      if (!(await askConfirm(tt.discardConfirm, tt.discardOk, tt.cancel))) return;
    }
    dirtyRef.current = false;
    setEditorShot(next);
  }

  const capture = (kind: 'region' | 'window' | 'screen') => {
    api.capture(kind, delay).catch(showError);
  };

  const doImport = async () => {
    try {
      const picked = await open({
        multiple: true,
        filters: [{ name: t.imagesFilter, extensions: ['png', 'jpg', 'jpeg', 'PNG', 'JPG', 'JPEG'] }],
      });
      if (!picked) return;
      const paths = Array.isArray(picked) ? picked : [picked];
      const n = await api.importFiles(paths as string[]);
      showToast(n > 0 ? `${n} ${t.imported}` : t.importNone);
    } catch (e) {
      showError(e);
    }
  };

  // Standard-Verzögerung aus den Einstellungen (0–60 s) muss im Dropdown
  // auftauchen, sonst zeigt es „sofort“, während z. B. 7 s gelten
  const delayOptions = Array.from(new Set([0, 3, 5, 10, delay])).sort((a, b) => a - b);

  const filters: { key: Filter; label: string }[] = [
    { key: 'all', label: t.all },
    { key: 'favorites', label: t.favorites },
    { key: 'region', label: t.region },
    { key: 'window', label: t.window },
    { key: 'screen', label: t.screen },
    { key: 'edits', label: t.edits },
  ];

  if (!settings) return null;

  return (
    <div className="app">
      <div className="header">
        <div className="brand">
          <span className="name">screencap</span>
          <span className="dot">.</span>
        </div>
        {!editorShot && (
          <>
            <input
              className="grow"
              type="text"
              placeholder={t.search}
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
            <div className="capgroup">
              <button className="primary" onClick={() => capture('region')}>
                <IconRegion /> {t.captureRegion}
              </button>
              <button onClick={() => capture('window')}>
                <IconWindow /> {t.captureWindow}
              </button>
              <button onClick={() => capture('screen')}>
                <IconScreen /> {t.captureScreen}
              </button>
              <select
                title={t.delay}
                value={delay}
                onChange={(e) => setDelay(Number(e.target.value))}
              >
                {delayOptions.map((d) => (
                  <option key={d} value={d}>
                    {d === 0 ? t.delayNone : `${d} ${t.seconds}`}
                  </option>
                ))}
              </select>
            </div>
            <button className="ghost" title={t.importBtn} onClick={doImport}>
              <IconImport />
            </button>
            <button className="ghost" title={t.settings} onClick={() => setShowSettings(true)}>
              <IconGear />
            </button>
          </>
        )}
      </div>

      {updateAvail && (
        <div className="upd-banner">
          <span>
            {t.updateBanner} <strong>{updateAvail.version}</strong>
          </span>
          <button
            className="primary"
            disabled={installing}
            onClick={() => {
              setInstalling(true);
              api.installUpdate().catch((e) => {
                setInstalling(false);
                showError(e); // Backend-Meldung trägt schon „Update fehlgeschlagen: …“
              });
            }}
          >
            {installing ? t.updateInstalling : t.updateInstall}
          </button>
          <button className="ghost" onClick={() => setUpdateAvail(null)}>
            {t.updateLater}
          </button>
        </div>
      )}


      {editorShot ? (
        <Editor
          shot={editorShot}
          t={t}
          onBack={() => openEditor(null)}
          onShotChanged={(s) => setEditorShot(s)}
          onDirtyChange={(d) => {
            dirtyRef.current = d;
          }}
          showToast={showToast}
          showError={showError}
        />
      ) : (
        <>
          <div className="filters">
            {filters.map((f) => (
              <button
                key={f.key}
                className={`chip ${filter === f.key ? 'active' : ''}`}
                onClick={() => setFilter(f.key)}
              >
                {f.label}
              </button>
            ))}
            <span className="count">
              {shots.length} {t.shots}
            </span>
          </div>
          <div className="gallery">
            {shots.length === 0 && (
              <div className="empty">{query || filter !== 'all' ? t.emptyFiltered : t.empty}</div>
            )}
            {shots.map((s) => (
              <div key={s.id} className="card" onClick={() => openEditor(s)}>
                <div className="thumbwrap">
                  <Thumb shot={s} />
                  {s.favorite && (
                    <span className="fav">
                      <IconStar filled size={16} />
                    </span>
                  )}
                  <span className="hoveractions">
                    <button
                      className="icon"
                      title={t.copy}
                      onClick={(e) => {
                        e.stopPropagation();
                        api
                          .copyShot(s.id)
                          .then(() => showToast(t.copied))
                          .catch(showError);
                      }}
                    >
                      <IconCopy />
                    </button>
                    <button
                      className="icon"
                      title={t.favorite}
                      onClick={(e) => {
                        e.stopPropagation();
                        api.setFavorite(s.id, !s.favorite).catch(showError);
                      }}
                    >
                      <IconStar filled={s.favorite} />
                    </button>
                    <button
                      className="icon"
                      title={revealLabel}
                      onClick={(e) => {
                        e.stopPropagation();
                        api.revealShot(s.id).catch(showError);
                      }}
                    >
                      <IconFolder />
                    </button>
                    <button
                      className="icon"
                      title={t.delete}
                      onClick={async (e) => {
                        e.stopPropagation();
                        if (await askConfirm(t.deleteConfirm, t.deleteOk, t.cancel))
                          api.deleteShot(s.id).catch(showError);
                      }}
                    >
                      <IconTrash />
                    </button>
                  </span>
                </div>
                <div className="info">
                  <div className="name">{s.name}</div>
                  <div className="meta">
                    <span>
                      {s.width}×{s.height}
                    </span>
                    <span>{fmtSize(s.sizeBytes)}</span>
                    <span>{fmtDate(s.capturedAt, lang)}</span>
                  </div>
                  {s.tags.length > 0 && (
                    <div className="tags">
                      {s.tags.map((tag) => (
                        <span key={tag} className="tagchip">
                          {tag}
                        </span>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            ))}
          </div>
        </>
      )}

      {showSettings && (
        <SettingsModal
          settings={settings}
          t={t}
          onClose={() => setShowSettings(false)}
          onSave={async (s) => {
            // Fehler (z. B. Kürzel belegt, Ordner nicht beschreibbar) zeigt der Dialog an
            await api.setSettings(s);
            const saved = await api.getSettings();
            setSettings(saved);
            setDelay(saved.delayDefault);
            setShowSettings(false);
            thumbCache.clear();
            refresh();
          }}
        />
      )}

      <Help lang={lang} />
      {toast && (
        <div
          className={`toast ${toast.error ? 'error' : ''}`}
          role={toast.error ? 'alert' : 'status'}
          title={toast.error ? t.dismiss : undefined}
          onClick={() => setToast(null)}
        >
          {toast.msg}
        </div>
      )}
    </div>
  );
}
