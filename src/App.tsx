import { useCallback, useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import { api, Filter, Settings, Shot, UpdateInfo } from './api';
import { dicts, Lang } from './i18n';
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

function Thumb({ id }: { id: string }) {
  const [src, setSrc] = useState<string | null>(thumbCache.get(id) ?? null);
  useEffect(() => {
    let alive = true;
    if (thumbCache.has(id)) {
      setSrc(thumbCache.get(id)!);
      return;
    }
    api
      .getThumb(id)
      .then((data) => {
        thumbCache.set(id, data);
        if (alive) setSrc(data);
      })
      .catch(() => {});
    return () => {
      alive = false;
    };
  }, [id]);
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
  const [toast, setToast] = useState<string | null>(null);
  const [updateAvail, setUpdateAvail] = useState<UpdateInfo | null>(null);
  const [installing, setInstalling] = useState(false);
  const toastTimer = useRef<number | undefined>(undefined);
  const shotsRef = useRef<Shot[]>([]);
  shotsRef.current = shots;

  const lang: Lang = settings?.language ?? 'de';
  const t = dicts[lang];

  const showToast = useCallback((msg: string) => {
    setToast(msg);
    window.clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToast(null), 1500);
  }, []);

  const refresh = useCallback(() => {
    api.listShots(query, filter).then(setShots).catch(() => {});
  }, [query, filter]);

  useEffect(() => {
    api.getSettings().then((s) => {
      setSettings(s);
      setDelay(s.delayDefault);
    });
    // stiller Update-Check beim Start — installiert wird nur nach Klick
    api.checkUpdate().then(setUpdateAvail).catch(() => {});
  }, []);

  useEffect(refresh, [refresh]);

  useEffect(() => {
    const subs = [
      listen('library-changed', refresh),
      listen<string>('open-editor', (e) => {
        // the library event may arrive after this one — fetch fresh
        api.listShots('', 'all').then((all) => {
          const s = all.find((x) => x.id === e.payload);
          if (s) setEditorShot(s);
        });
      }),
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

  const capture = (kind: 'region' | 'window' | 'screen') => {
    api.capture(kind, delay).catch((e) => showToast(String(e)));
  };

  const doImport = async () => {
    const picked = await open({
      multiple: true,
      filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp'] }],
    });
    if (!picked) return;
    const paths = Array.isArray(picked) ? picked : [picked];
    const n = await api.importFiles(paths as string[]);
    showToast(`${n} ${t.imported}`);
  };

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
                <option value={0}>{t.delayNone}</option>
                <option value={3}>3 s</option>
                <option value={5}>5 s</option>
                <option value={10}>10 s</option>
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

      {editorShot ? (
        <Editor
          shot={editorShot}
          t={t}
          onBack={() => setEditorShot(null)}
          onShotChanged={(s) => setEditorShot(s)}
          showToast={showToast}
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
              <div key={s.id} className="card" onClick={() => setEditorShot(s)}>
                <div className="thumbwrap">
                  <Thumb id={s.id} />
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
                        api.copyShot(s.id).then(() => showToast(t.copied));
                      }}
                    >
                      <IconCopy />
                    </button>
                    <button
                      className="icon"
                      title={t.favorite}
                      onClick={(e) => {
                        e.stopPropagation();
                        api.setFavorite(s.id, !s.favorite);
                      }}
                    >
                      <IconStar filled={s.favorite} />
                    </button>
                    <button
                      className="icon"
                      title={t.reveal}
                      onClick={(e) => {
                        e.stopPropagation();
                        api.revealShot(s.id);
                      }}
                    >
                      <IconFolder />
                    </button>
                    <button
                      className="icon"
                      title={t.delete}
                      onClick={async (e) => {
                        e.stopPropagation();
                        const { confirm } = await import('@tauri-apps/plugin-dialog');
                        if (await confirm(t.deleteConfirm)) api.deleteShot(s.id);
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
          onSave={(s) => {
            api.setSettings(s).then(() => {
              setSettings(s);
              setDelay(s.delayDefault);
              setShowSettings(false);
              thumbCache.clear();
              refresh();
            });
          }}
        />
      )}

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
              api.installUpdate().catch(() => setInstalling(false));
            }}
          >
            {installing ? t.updateInstalling : t.updateInstall}
          </button>
          <button className="ghost" onClick={() => setUpdateAvail(null)}>
            {t.updateLater}
          </button>
        </div>
      )}

      <Help lang={lang} />
      {toast && <div className="toast">{toast}</div>}
    </div>
  );
}
