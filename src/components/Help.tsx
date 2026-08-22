import { useState } from 'react';
import { Lang } from '../i18n';

// Selbstständiges Hilfe-System: schwebender ?-Button, First-Run-Tutorial
// und durchsuchbares Handbuch.

interface Step {
  title: string;
  body: string[];
}

interface Section {
  id: string;
  title: string;
  body: string[];
}

interface Content {
  labels: {
    fab: string;
    tutorial: string;
    manual: string;
    search: string;
    next: string;
    back: string;
    skip: string;
    done: string;
    stepOf: (n: number, total: number) => string;
    noResults: string;
  };
  tutorial: Step[];
  sections: Section[];
}

const de: Content = {
  labels: {
    fab: 'Hilfe & Handbuch',
    tutorial: 'Tutorial',
    manual: 'Handbuch',
    search: 'Handbuch durchsuchen …',
    next: 'Weiter',
    back: 'Zurück',
    skip: 'Überspringen',
    done: 'Los geht’s',
    stepOf: (n, total) => `Schritt ${n} von ${total}`,
    noResults: 'Keine Treffer',
  },
  tutorial: [
    {
      title: 'Willkommen bei screencap.',
      body: [
        'screencap nimmt Screenshots auf, öffnet direkt den Editor — und legt alles durchsuchbar in einer Bibliothek ab.',
        'Deine Aufnahmen bleiben normale Bilddateien auf deinem Rechner. Kein Upload, kein Konto, keine Telemetrie.',
        'Dieses Tutorial dauert eine Minute. Du findest es jederzeit wieder über den ?-Knopf unten rechts.',
      ],
    },
    {
      title: 'Aufnehmen',
      body: [
        'Drei Arten, oben rechts oder per Kürzel:',
        '• Ausschnitt (Cmd+Shift+7) — Bereich mit der Maus aufziehen',
        '• Fenster (Cmd+Shift+8) — Fenster anklicken, Esc bricht ab',
        '• Bildschirm (Cmd+Shift+9) — der ganze Monitor',
        'Das Dropdown daneben stellt eine Verzögerung ein (3/5/10 s) — praktisch für Menüs und Tooltips.',
        'Wichtig: Beim allerersten Mal fragt macOS nach der Bildschirmaufnahme-Freigabe — einmal erlauben, fertig.',
      ],
    },
    {
      title: 'Der Editor',
      body: [
        'Nach der Aufnahme öffnet sich der Editor automatisch (abschaltbar in den Einstellungen).',
        '• Pfeile, Linien, Rechtecke, Ellipsen, Freihand und Text — in 6 Farben und 3 Strichstärken',
        '• Auswahl-Werkzeug: jedes Objekt bleibt bis zum Speichern voll editierbar — verschieben, über Eck- und Kanten-Anfasser skalieren, dehnen und stauchen, Linien/Pfeile biegen, duplizieren (⌘D, ⌥-Ziehen), Farbe/Stärke nachträglich wechseln',
        '• Verpixeln: Bereich aufziehen — beim Speichern fest ins Bild gerechnet, keine abnehmbare Ebene',
        '• Zuschneiden: Bereich wählen, „Zuschnitt anwenden“',
        'Undo/Redo jederzeit; gespeichert wird erst, wenn du „Speichern“ oder „Als Kopie speichern“ klickst.',
      ],
    },
    {
      title: 'Die Bibliothek',
      body: [
        'Jede Aufnahme landet als Karte in der Galerie — mit Thumbnail, Name, Größe und Datum.',
        '• Suche findet Namen, Dateinamen und Tags',
        '• Filter-Chips: Favoriten, Ausschnitte, Fenster, Bildschirme, Bearbeitet',
        '• Karten-Aktionen (bei Maus darüber): Kopieren, Favorit, Im Finder zeigen, Löschen',
        'Klick auf eine Karte öffnet den Editor mit Name, Tags und Details.',
      ],
    },
    {
      title: 'Ablage-Ordner',
      body: [
        'Alles liegt in ~/Pictures/screencap/ — als ganz normale PNG- oder JPG-Dateien.',
        'Der Ordner ist in den Einstellungen frei wählbar. Dateien, die du selbst hineinlegst, übernimmt screencap automatisch in die Bibliothek; gelöschte verschwinden daraus.',
      ],
    },
    {
      title: 'Tray & Fenster',
      body: [
        'Das Schließen des Fensters versteckt es nur — die Kürzel funktionieren weiter. Beenden geht über das Tray-Menü.',
        'Aus dem Tray heraus kannst du auch direkt aufnehmen: Ausschnitt, Fenster oder Bildschirm.',
      ],
    },
  ],
  sections: [
    {
      id: 'capture',
      title: 'Aufnehmen',
      body: [
        'Drei Aufnahmearten — per Button in der Kopfzeile, über das Tray-Menü oder global per Kürzel:',
        '• Ausschnitt (Cmd/Ctrl+Shift+7) — Bereich mit der Maus aufziehen; Esc bricht ab',
        '• Fenster (Cmd/Ctrl+Shift+8) — gewünschtes Fenster anklicken; mit Fensterschatten (abschaltbar)',
        '• Bildschirm (Cmd/Ctrl+Shift+9) — der komplette Monitor, sofort',
        'Verzögerung: Das Dropdown neben den Buttons wartet 3, 5 oder 10 Sekunden vor der Aufnahme — Zeit, um Menüs zu öffnen. Die Standard-Verzögerung für Kürzel stellst du in den Einstellungen ein.',
        'Das screencap-Fenster versteckt sich während der Aufnahme automatisch, damit es nicht mit aufs Bild kommt.',
        'macOS fragt bei der ersten Aufnahme nach der Bildschirmaufnahme-Freigabe (Systemeinstellungen → Datenschutz & Sicherheit → Bildschirmaufnahme) — einmal erlauben genügt.',
      ],
    },
    {
      id: 'editor',
      title: 'Editor',
      body: [
        'Der Editor öffnet nach jeder Aufnahme (Einstellung „Editor nach der Aufnahme öffnen“) oder per Klick auf eine Karte in der Bibliothek.',
        'Werkzeuge in der Leiste:',
        '• Auswählen & Verschieben — Objekt anklicken: Ziehen verschiebt es; die 8 Eck-/Kanten-Anfasser skalieren, dehnen und stauchen jedes Objekt (auch Freihand und Text), bei Linien/Pfeilen sitzen Anfasser an den Endpunkten plus ein runder Biege-Anfasser in der Mitte (zurück zur Geraden: nahe an die Mitte ziehen). Farbe/Strichstärke in der Leiste wirken aufs markierte Objekt. Kopieren: ⌘D oder ⌘C/⌘V dupliziert, ⌥-Ziehen zieht eine Kopie ab. Doppelklick auf Text bearbeitet ihn, Entf/Backspace löscht, Pfeiltasten verschieben pixelweise (Shift = 10 px), Esc: Auswahl aufheben bzw. zurück zur Bibliothek.',
        '• Stift — Freihand zeichnen',
        '• Linie / Pfeil — gerade Verbindungen, Pfeil mit Spitze',
        '• Rechteck / Ellipse — Rahmen um Wichtiges',
        '• Text — klicken, tippen, Enter bestätigt (Shift+Enter für Zeilenumbruch, Esc bricht ab)',
        '• Verpixeln — Bereich aufziehen; wird beim Speichern unwiderruflich ins Bild gerechnet',
        '• Zuschneiden — Bereich wählen, dann „Zuschnitt anwenden“',
        'Dazu: 6 Farben, 3 Strichstärken (2/4/8 px), Undo/Redo.',
        '„Speichern“ überschreibt die Datei, „Als Kopie speichern“ legt eine neue Datei an (Typ „Bearbeitet“) und lässt das Original unangetastet.',
      ],
    },
    {
      id: 'pixelate',
      title: 'Verpixeln & sensible Inhalte',
      body: [
        'Verpixeln ist destruktiv gemeint: Beim Speichern wird der Bereich fest ins Bild gerechnet — es gibt keine abnehmbare Ebene, aus der sich der Inhalt wiederherstellen ließe.',
        'Bis zum Speichern bleibt der Effekt eine Vorschau und lässt sich per Undo entfernen.',
        'Tipp: Wer das Original behalten will, nutzt „Als Kopie speichern“ und teilt nur die verpixelte Kopie.',
      ],
    },
    {
      id: 'library',
      title: 'Bibliothek',
      body: [
        'Die Galerie zeigt alle Aufnahmen als Karten, neueste zuerst.',
        '• Suche — findet Namen, Dateinamen und Tags',
        '• Filter — Alles, Favoriten, Ausschnitte, Fenster, Bildschirme, Bearbeitet',
        '• Karten-Aktionen bei Maus darüber: Kopieren, Favorit, Im Finder zeigen, Löschen',
        'Im Editor rechts bearbeitest du Name und Tags (Komma-getrennt) und siehst Auflösung, Dateigröße und Datum.',
        'Favoriten (Stern) heben Wichtiges hervor und haben einen eigenen Filter.',
      ],
    },
    {
      id: 'folder',
      title: 'Ablage-Ordner & Import',
      body: [
        'Standard-Ablage ist ~/Pictures/screencap/ — änderbar in den Einstellungen („Ablage-Ordner“).',
        'Die Aufnahmen sind normale PNG-/JPG-Dateien; der Bibliotheks-Index (index.json) und die Thumbnails (.thumbs/) liegen daneben.',
        'Der Ordner wird mit der Wirklichkeit abgeglichen: Bilddateien, die du selbst hineinkopierst, tauchen als „Import“ in der Bibliothek auf; von Hand gelöschte Dateien verschwinden aus dem Index.',
        'Import-Knopf in der Kopfzeile: bestehende Bilder (PNG, JPG, WebP) auswählen — sie werden in den Ablage-Ordner kopiert und aufgenommen.',
      ],
    },
    {
      id: 'shortcuts',
      title: 'Kürzel',
      body: [
        '• Cmd/Ctrl+Shift+7 — Ausschnitt aufnehmen',
        '• Cmd/Ctrl+Shift+8 — Fenster aufnehmen',
        '• Cmd/Ctrl+Shift+9 — Bildschirm aufnehmen',
        'Alle drei sind global (funktionieren aus jeder App) und in den Einstellungen frei belegbar — Format z. B. „CmdOrCtrl+Shift+7“.',
      ],
    },
    {
      id: 'settings',
      title: 'Einstellungen',
      body: [
        '• Sprache — Deutsch / English',
        '• Ablage-Ordner — wohin Aufnahmen gespeichert werden (leer = ~/Pictures/screencap)',
        '• Format — PNG (verlustfrei) oder JPG (kleiner)',
        '• Standard-Verzögerung — gilt für Kürzel- und Tray-Aufnahmen',
        '• Aufnahme zusätzlich in die Zwischenablage — direkt einfügbar nach jedem Schuss',
        '• Editor nach der Aufnahme öffnen — an/aus',
        '• Fensterschatten mit aufnehmen — der macOS-Schatten bei Fenster-Aufnahmen',
        '• Beim Anmelden starten — screencap automatisch mit dem System starten',
        '• Kürzel — alle drei Aufnahme-Kürzel frei belegen',
      ],
    },
    {
      id: 'updates',
      title: 'Updates',
      body: [
        'screencap prüft beim Start automatisch auf neue Versionen und zeigt einen Hinweis, wenn eine bereitsteht — installiert wird erst nach deinem Klick.',
        'Manuell prüfen: Einstellungen → „Nach Updates suchen“. Vor der Installation siehst du das Changelog.',
        'Updates kommen signiert von GitHub (LAN-SOLO/screencap): Die App prüft die Signatur, bevor irgendetwas installiert wird. Bibliothek und Einstellungen bleiben unangetastet.',
      ],
    },
    {
      id: 'privacy',
      title: 'Privatsphäre',
      body: [
        'Alles bleibt lokal: Aufnahmen, Bibliothek und Einstellungen liegen auf deinem Rechner. Kein Upload, kein Konto, keine Telemetrie.',
        'Die einzige Netzwerkverbindung ist der Update-Check gegen GitHub.',
      ],
    },
    {
      id: 'tray',
      title: 'Tray & Fensterverhalten',
      body: [
        'Das Schließen des Fensters (rotes X) versteckt es nur — die globalen Kürzel funktionieren weiter.',
        'Tray-Menü: Ausschnitt/Fenster/Bildschirm aufnehmen, screencap öffnen, screencap beenden.',
        'Auf dem Mac holt auch ein Klick auf das Dock-Symbol das Fenster zurück.',
      ],
    },
  ],
};

const en: Content = {
  labels: {
    fab: 'Help & manual',
    tutorial: 'Tutorial',
    manual: 'Manual',
    search: 'Search the manual …',
    next: 'Next',
    back: 'Back',
    skip: 'Skip',
    done: 'Let’s go',
    stepOf: (n, total) => `Step ${n} of ${total}`,
    noResults: 'No matches',
  },
  tutorial: [
    {
      title: 'Welcome to screencap.',
      body: [
        'screencap takes screenshots, opens the editor right away — and files everything into a searchable library.',
        'Your captures stay ordinary image files on your machine. No upload, no account, no telemetry.',
        'This tutorial takes a minute. Reopen it anytime via the ? button in the bottom right.',
      ],
    },
    {
      title: 'Capturing',
      body: [
        'Three modes, top right or via shortcut:',
        '• Region (Cmd+Shift+7) — drag an area with the mouse',
        '• Window (Cmd+Shift+8) — click a window, Esc cancels',
        '• Screen (Cmd+Shift+9) — the whole monitor',
        'The dropdown next to the buttons sets a delay (3/5/10 s) — handy for menus and tooltips.',
        'Note: on the very first capture, macOS asks for screen-recording permission — allow once, done.',
      ],
    },
    {
      title: 'The editor',
      body: [
        'After a capture the editor opens automatically (can be turned off in Settings).',
        '• Arrows, lines, rectangles, ellipses, freehand and text — 6 colors, 3 stroke widths',
        '• Select tool: every object stays fully editable until you save — move, scale/stretch/squash via corner and edge handles, bend lines/arrows, duplicate (⌘D, ⌥-drag), change color/width afterwards',
        '• Pixelate: drag an area — baked into the image on save, no removable layer',
        '• Crop: select an area, then “Apply crop”',
        'Undo/redo anytime; nothing is written until you hit “Save” or “Save as copy”.',
      ],
    },
    {
      title: 'The library',
      body: [
        'Every capture becomes a card in the gallery — thumbnail, name, size and date.',
        '• Search matches names, file names and tags',
        '• Filter chips: Favorites, Regions, Windows, Screens, Edited',
        '• Card actions on hover: copy, favorite, reveal in Finder, delete',
        'Clicking a card opens the editor with name, tags and details.',
      ],
    },
    {
      title: 'The library folder',
      body: [
        'Everything lives in ~/Pictures/screencap/ — as plain PNG or JPG files.',
        'The folder is configurable in Settings. Files you drop in yourself are adopted into the library automatically; deleted files disappear from it.',
      ],
    },
    {
      title: 'Tray & window',
      body: [
        'Closing the window only hides it — the shortcuts keep working. Quit via the tray menu.',
        'You can also capture straight from the tray: region, window or screen.',
      ],
    },
  ],
  sections: [
    {
      id: 'capture',
      title: 'Capturing',
      body: [
        'Three capture modes — via header buttons, the tray menu or global shortcuts:',
        '• Region (Cmd/Ctrl+Shift+7) — drag an area with the mouse; Esc cancels',
        '• Window (Cmd/Ctrl+Shift+8) — click the window you want; with window shadow (optional)',
        '• Screen (Cmd/Ctrl+Shift+9) — the whole monitor, instantly',
        'Delay: the dropdown next to the buttons waits 3, 5 or 10 seconds before capturing — time to open menus. The default delay for shortcuts is set in Settings.',
        'The screencap window hides itself during capture so it doesn’t end up in the shot.',
        'macOS asks for screen-recording permission on the first capture (System Settings → Privacy & Security → Screen Recording) — allowing once is enough.',
      ],
    },
    {
      id: 'editor',
      title: 'Editor',
      body: [
        'The editor opens after every capture (setting “Open editor after capture”) or by clicking a card in the library.',
        'Tools in the bar:',
        '• Select & move — click an object: drag moves it; the 8 corner/edge handles scale, stretch and squash any object (freehand and text included), lines/arrows get endpoint handles plus a round bend handle in the middle (drag it near the midpoint to snap back straight). Color/width in the bar apply to the selected object. Copying: ⌘D or ⌘C/⌘V duplicates, ⌥-drag peels off a copy. Double-click text to edit it, Del/Backspace deletes, arrow keys nudge by a pixel (Shift = 10 px), Esc: deselect or back to the library.',
        '• Pen — freehand drawing',
        '• Line / Arrow — straight connections, arrow with head',
        '• Rectangle / Ellipse — frames around what matters',
        '• Text — click, type, Enter confirms (Shift+Enter for line breaks, Esc cancels)',
        '• Pixelate — drag an area; permanently baked into the image on save',
        '• Crop — select an area, then “Apply crop”',
        'Plus: 6 colors, 3 stroke widths (2/4/8 px), undo/redo.',
        '“Save” overwrites the file, “Save as copy” creates a new file (kind “Edited”) and leaves the original untouched.',
      ],
    },
    {
      id: 'pixelate',
      title: 'Pixelation & sensitive content',
      body: [
        'Pixelation is meant to be destructive: on save, the area is baked into the image — there is no removable layer the content could be recovered from.',
        'Until you save, the effect is a preview and can be removed with undo.',
        'Tip: to keep the original, use “Save as copy” and share only the pixelated copy.',
      ],
    },
    {
      id: 'library',
      title: 'Library',
      body: [
        'The gallery shows all captures as cards, newest first.',
        '• Search — matches names, file names and tags',
        '• Filters — Everything, Favorites, Regions, Windows, Screens, Edited',
        '• Card actions on hover: copy, favorite, reveal in Finder, delete',
        'In the editor’s side panel you edit name and tags (comma-separated) and see resolution, file size and date.',
        'Favorites (star) highlight what matters and have their own filter.',
      ],
    },
    {
      id: 'folder',
      title: 'Library folder & import',
      body: [
        'The default location is ~/Pictures/screencap/ — changeable in Settings (“Library folder”).',
        'Captures are plain PNG/JPG files; the library index (index.json) and thumbnails (.thumbs/) live next to them.',
        'The folder is reconciled with reality: image files you copy in yourself appear as “Import” in the library; files deleted by hand disappear from the index.',
        'Import button in the header: pick existing images (PNG, JPG, WebP) — they are copied into the library folder and adopted.',
      ],
    },
    {
      id: 'shortcuts',
      title: 'Shortcuts',
      body: [
        '• Cmd/Ctrl+Shift+7 — capture region',
        '• Cmd/Ctrl+Shift+8 — capture window',
        '• Cmd/Ctrl+Shift+9 — capture screen',
        'All three are global (they work from any app) and freely configurable in Settings — e.g. “CmdOrCtrl+Shift+7”.',
      ],
    },
    {
      id: 'settings',
      title: 'Settings',
      body: [
        '• Language — Deutsch / English',
        '• Library folder — where captures are stored (empty = ~/Pictures/screencap)',
        '• Format — PNG (lossless) or JPG (smaller)',
        '• Default delay — applies to shortcut and tray captures',
        '• Also copy captures to the clipboard — paste right after every shot',
        '• Open editor after capture — on/off',
        '• Include window shadow — the macOS shadow on window captures',
        '• Start at login — launch screencap with the system',
        '• Shortcuts — all three capture shortcuts, freely configurable',
      ],
    },
    {
      id: 'updates',
      title: 'Updates',
      body: [
        'screencap checks for new versions on launch and shows a notice when one is available — nothing installs without your click.',
        'Check manually: Settings → “Check for updates”. You see the changelog before installing.',
        'Updates come signed from GitHub (LAN-SOLO/screencap): the app verifies the signature before installing anything. Library and settings stay untouched.',
      ],
    },
    {
      id: 'privacy',
      title: 'Privacy',
      body: [
        'Everything stays local: captures, library and settings live on your machine. No upload, no account, no telemetry.',
        'The only network connection is the update check against GitHub.',
      ],
    },
    {
      id: 'tray',
      title: 'Tray & window behavior',
      body: [
        'Closing the window (red X) only hides it — the global shortcuts keep working.',
        'Tray menu: capture region/window/screen, open screencap, quit screencap.',
        'On the Mac, clicking the dock icon brings the window back.',
      ],
    },
  ],
};

const SEEN_KEY = 'screencap.tutorialSeen';

export function Help({ lang }: { lang: Lang }) {
  const c = lang === 'de' ? de : en;
  const [mode, setMode] = useState<'closed' | 'tutorial' | 'manual'>(() =>
    localStorage.getItem(SEEN_KEY) ? 'closed' : 'tutorial'
  );
  const [step, setStep] = useState(0);
  const [sel, setSel] = useState(c.sections[0].id);
  const [q, setQ] = useState('');

  const close = () => {
    localStorage.setItem(SEEN_KEY, '1');
    setMode('closed');
    setStep(0);
  };

  const query = q.trim().toLowerCase();
  const filtered = query
    ? c.sections.filter(
        (s) =>
          s.title.toLowerCase().includes(query) ||
          s.body.some((p) => p.toLowerCase().includes(query))
      )
    : c.sections;
  const current = filtered.find((s) => s.id === sel) ?? filtered[0] ?? null;

  return (
    <>
      <button className="hlp-fab" title={c.labels.fab} onClick={() => setMode('manual')}>
        ?
      </button>
      {mode !== 'closed' && (
        <div className="hlp-overlay" onClick={close}>
          <div className="hlp-modal" onClick={(e) => e.stopPropagation()}>
            <div className="hlp-head">
              <span className="hlp-brand">
                <span className="hlp-name">screencap</span>
                <span className="hlp-dot">.</span>
              </span>
              <button
                className={`hlp-tab ${mode === 'tutorial' ? 'active' : ''}`}
                onClick={() => {
                  setMode('tutorial');
                  setStep(0);
                }}
              >
                {c.labels.tutorial}
              </button>
              <button
                className={`hlp-tab ${mode === 'manual' ? 'active' : ''}`}
                onClick={() => setMode('manual')}
              >
                {c.labels.manual}
              </button>
              <span className="hlp-spacer" />
              <button className="hlp-close" onClick={close}>
                ✕
              </button>
            </div>

            {mode === 'tutorial' && (
              <div className="hlp-tut">
                <div className="hlp-step-count">
                  {c.labels.stepOf(step + 1, c.tutorial.length)}
                </div>
                <h2>{c.tutorial[step].title}</h2>
                {c.tutorial[step].body.map((p, i) =>
                  p.startsWith('• ') ? (
                    <div key={i} className="hlp-li">
                      {p.slice(2)}
                    </div>
                  ) : (
                    <p key={i}>{p}</p>
                  )
                )}
                <div className="hlp-tut-nav">
                  <button className="hlp-ghost" onClick={close}>
                    {c.labels.skip}
                  </button>
                  <span className="hlp-dots">
                    {c.tutorial.map((_, i) => (
                      <span key={i} className={i === step ? 'on' : ''} />
                    ))}
                  </span>
                  {step > 0 && (
                    <button onClick={() => setStep(step - 1)}>{c.labels.back}</button>
                  )}
                  {step < c.tutorial.length - 1 ? (
                    <button className="hlp-primary" onClick={() => setStep(step + 1)}>
                      {c.labels.next}
                    </button>
                  ) : (
                    <button className="hlp-primary" onClick={close}>
                      {c.labels.done}
                    </button>
                  )}
                </div>
              </div>
            )}

            {mode === 'manual' && (
              <div className="hlp-body">
                <div className="hlp-toc">
                  <input
                    type="text"
                    placeholder={c.labels.search}
                    value={q}
                    onChange={(e) => setQ(e.target.value)}
                  />
                  {filtered.length === 0 && (
                    <div className="hlp-empty">{c.labels.noResults}</div>
                  )}
                  {filtered.map((s) => (
                    <button
                      key={s.id}
                      className={`hlp-toc-item ${current?.id === s.id ? 'active' : ''}`}
                      onClick={() => setSel(s.id)}
                    >
                      {s.title}
                    </button>
                  ))}
                </div>
                <div className="hlp-content">
                  {current && (
                    <>
                      <h2>{current.title}</h2>
                      {current.body.map((p, i) =>
                        p.startsWith('• ') ? (
                          <div key={i} className="hlp-li">
                            {p.slice(2)}
                          </div>
                        ) : (
                          <p key={i}>{p}</p>
                        )
                      )}
                    </>
                  )}
                </div>
              </div>
            )}
          </div>
        </div>
      )}
    </>
  );
}
