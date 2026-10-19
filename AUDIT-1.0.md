# screencap. — Audit zur Version 1.0.0

Stand: 2026-10-19 · Basis: `1ffb614` (0.2.0) · Ziel: Test-Build 1.0.0 für macOS + Windows

Dieses Dokument hält fest, was für 1.0 geprüft, getestet und behoben wurde, was
offen bleibt und was vor der Freigabe noch von Hand auf echten Rechnern zu
prüfen ist.

**Legende Teststatus**

- **A** — automatisiert getestet (`cargo test --workspace`)
- **U** — in der echten React-Oberfläche getestet: Vite-Dev-Server, Headless-Chrome
  über das DevTools-Protokoll, Tauri-IPC per `@tauri-apps/api/mocks` simuliert
  (Mausziehen, Tastenkürzel, Dialog-Antworten Ja/Nein, macOS- und Windows-User-Agent)
- **S** — nur statisch geprüft (Code-Review, Aufrufpfade Frontend → Rust)
- **M** — braucht einen Menschen am echten System (siehe Checkliste unten)

## 1. Funktionsinventar

### Aufnahme

| Funktion | Status | Anmerkung |
|---|---|---|
| Ausschnitt / Fenster / Bildschirm per Button | S, M | macOS `screencapture -i` / `-i -W` / Vollbild |
| Globale Kürzel `CmdOrCtrl+Shift+7/8/9` | A (Parsing/Dubletten), M | Registrierung braucht echtes OS |
| Tray-Menü: 3 Aufnahmen, Öffnen, Beenden | S, M | Beschriftung wechselt jetzt sofort mit der Sprache |
| Verzögerung 0/3/5/10 s, Standard-Verzögerung | A (Begrenzung 60 s), S | macOS `-T`, Windows `sleep` |
| Esc bricht ab (keine Fehlermeldung) | A (`capture_outcome`) | vorher Fehler-Toast bei Esc auf macOS |
| eigenes Fenster wird vor der Aufnahme versteckt | S, M | 350 ms Wartezeit |
| Windows-Ausschnitt über Overlay `region` | U (Ziehen, DPR 1,5 → physische Pixel, Esc, Rechtsklick, Klick ohne Ziehen), S, M | Clamping auf Monitorgröße in Rust |
| Windows-Fenster = Vordergrundfenster, Bildschirm = Monitor unter Mauszeiger | S, M | `xcap 0.9` |
| Linux | — | keine Aufnahme (dokumentiert, Fehlermeldung) |
| Zwischenablage nach Aufnahme, Editor nach Aufnahme | S, M | |
| Dateiname `screencap-JJJJMMTT-HHMMSS[-n].png/jpg` | A (`unique_file_name`) | |
| Thumbnail ≤ 512 px | A (`thumbnail_is_bounded`) | |

### Editor

| Funktion | Status | Anmerkung |
|---|---|---|
| Stift, Linie, Pfeil, Rechteck, Ellipse | U (Rechteck, Stift, Pfeil) | |
| Text (Enter, Shift+Enter, Esc) | U | offenes Textfeld wird jetzt beim Speichern/Werkzeugwechsel übernommen (Pixeltest im gespeicherten PNG) |
| Verpixeln (destruktiv beim Speichern) | U | Vorschau sichtbar, im gespeicherten Bild fest |
| Zuschneiden | U | 800 px → 400 px, gespeichertes PNG hat Zuschnittgröße |
| Auswahl: Verschieben, Entf | U | |
| Anfasser skalieren/dehnen, Biegen, Doppelklick-Textedit, ⌥-Ziehen, ⌘D/⌘C/⌘V, Pfeiltasten | S | unverändert seit 0.2.0 |
| Undo/Redo-Knöpfe + **neu** ⌘Z / ⌘⇧Z / Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y | U | |
| 6 Farben, 3 Stärken, Stil auf Auswahl | S | dunkles Farbfeld war im Dunkel-Modus unsichtbar → Rand |
| Speichern (Überschreiben) / Als Kopie speichern | U, A (Dateinamen) | |
| Verwerfen-Abfrage bei ungespeicherten Änderungen | U (Zurück, Esc, neue Aufnahme; Ja/Nein) | neu |
| Seitenleiste: Name, Tags, Favorit, Kopieren, Im Finder/Explorer zeigen, Löschen | U | Name/Tags folgen jetzt der angezeigten Aufnahme |

### Bibliothek

| Funktion | Status | Anmerkung |
|---|---|---|
| Galerie, neueste zuerst | A, U | |
| Suche (Name/Datei/Tags) | A, U | |
| Filter Alles/Favoriten/Ausschnitte/Fenster/Bildschirme/Bearbeitet | A, U | |
| Karten-Aktionen Kopieren/Favorit/Zeigen/Löschen | U (Favorit, Löschen), S (Kopieren, Zeigen) | Fehler werden jetzt angezeigt |
| Import (PNG/JPG) | S | Inhalt wird geprüft; WebP entfällt (siehe Bugs) |
| Disk-Sync: fremde Dateien = Import, gelöschte verschwinden | A | versteckte Dateien ignoriert |
| Index-Wiederherstellung bei kaputter `index.json` | A | |
| Ablage-Ordner wählen / wechseln | S | muss anlegbar sein |

### Einstellungen, System, Hilfe

| Funktion | Status | Anmerkung |
|---|---|---|
| Sprache DE/EN, erster Start = Systemsprache | U | GUI-Apps sehen kein `$LANG` → bisher immer Englisch |
| Modus Dunkel/Hell | U | |
| Format PNG/JPG, Standard-Verzögerung, Zwischenablage, Editor nach Aufnahme | A (Normalisierung), U (Speichern) | |
| Fensterschatten | S | nur macOS, unter Windows ausgeblendet |
| Autostart | S, M | Fehler wird vor dem Speichern gemeldet |
| Kürzel frei belegen | A (Parsing, Dubletten), U (Fehleranzeige im Dialog) | alte Kürzel bleiben bei Fehler aktiv |
| Update-Banner, „Nach Updates suchen“, Installieren | S, M | Version jetzt aus `getVersion()` |
| Hilfe: Tutorial, Handbuch, Suche, DE/EN-Umschalter | U | Inhalte für 1.0 ergänzt |
| Schließen = Verstecken, Dock-Klick holt Fenster zurück | S, M | |

## 2. Behobene Fehler

1. **Esc bei Ausschnitt/Fenster (macOS)** meldete „screencapture meldete einen Fehler“ — `screencapture` endet beim Abbruch mit Fehlercode. Jetzt: fehlende Datei bei interaktiver Aufnahme = Abbruch.
2. **Überschreiben einer JPG-Aufnahme** schrieb `<name>.png` — gehörte der Name einer anderen Aufnahme, wurde diese still überschrieben. Jetzt freier Name `-2`, `-3` ….
3. **„Als Kopie speichern“ → Namensfeld** zeigte weiter den alten Namen; beim Verlassen des Feldes wurde die Kopie auf den alten Namen umbenannt.
4. **Ungespeicherte Editor-Änderungen** gingen bei Zurück/Esc oder einer neuen Aufnahme ohne Rückfrage verloren.
5. **Offenes Textfeld** ging beim Speichern verloren.
6. **Veraltete Thumbnails** nach „Speichern“ (Cache nur nach ID).
7. **WebP-Import** ohne Decoder im Build: keine Thumbnails, 0×0, Kopieren schlug fehl. WebP wird nicht mehr angeboten (ein Decoder wäre eine neue Abhängigkeit).
8. **Einstellungen**: Fehler (Kürzel belegt/ungültig, Autostart) wurden nicht angezeigt, das Fenster blieb kommentarlos offen; ungültige Kürzel wurden trotzdem gespeichert und alle Kürzel abgemeldet. Jetzt: Prüfung vorher, Rollback, Fehlermeldung im Dialog.
9. **Unbekannte Sprache in settings.json** ließ die Oberfläche abstürzen (Wörterbuch `undefined`) → Normalisierung.
10. **Tray-Menü** blieb bis zum Neustart in der alten Sprache.
11. **Erster Start** immer Englisch (macOS/Windows setzen für GUI-Apps kein `LANG`).
12. **Fehlender Ablage-Ordner** (abgezogenes Laufwerk) brach den App-Start ab.
13. **Löschen unter Windows** bei geöffneter Datei: Eintrag weg, Datei blieb, nächster Sync importierte sie erneut.
14. Kürzel-Konflikte beim Start waren nur auf stderr sichtbar → Hinweis in den Einstellungen.
15. Hilfe/Labels macOS-lastig: „Im Finder zeigen“, ⌘-Hinweise, Fensterschatten unter Windows → plattformabhängig.
16. Fast schwarzes Farbfeld im Dunkel-Modus unsichtbar.
17. `localStorage`-Zugriff der Hilfe ohne try/catch.

## 3. Sicherheitsprüfung

| Bereich | Befund | Schwere | Status |
|---|---|---|---|
| Pfade aus `index.json` | Einträge wie `../../.ssh/id_rsa` wurden an den Ablage-Ordner gehängt; `get_image` hätte sie gelesen, `delete_shot` gelöscht (der Start-Sync verwarf sie meist vorher, aber nicht garantiert) | mittel | behoben: `is_safe_file_name` beim Laden, Test |
| Capabilities | `opener:default` (URLs öffnen) und `dialog:allow-save` ungenutzt; Overlay-Fenster hatte dieselben Rechte | niedrig | behoben: nur `dialog:allow-open`/`allow-message`, Overlay nur `core:default` |
| Editor-Daten (`save_edit`) | beliebige Bytes wurden als `.png` geschrieben, sofern decodierbar | niedrig | behoben: nur PNG |
| Import | Endung entschied, nicht der Inhalt; versteckte/Doppelpunkt-Namen | niedrig | behoben |
| Schreibvorgänge | Bild/Settings nicht atomar | niedrig | behoben (Temp + Rename) |
| CSP | `default-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:` — keine Remote-Quellen, kein `unsafe-eval` | — | ok |
| XSS | kein `dangerouslySetInnerHTML`, Namen/Tags als Text gerendert | — | ok |
| Kommandoausführung | nur `/usr/sbin/screencapture` mit festen Argumenten, Zielpfad als eigenes Argument (keine Shell) | — | ok |
| Updater | Minisign-Pubkey gesetzt, Endpoint HTTPS (GitHub), Installation nur nach Klick | — | ok |
| TLS | nirgends deaktiviert | — | ok |
| Secrets/Logs | keine Secrets in der App; Fehlermeldungen enthalten höchstens lokale Pfade | — | ok |
| localStorage | nur `theme` und `screencap.tutorialSeen` | — | ok |
| Symlinks in der Ablage | Sync nimmt nur reguläre Dateien | — | ok |
| `rustls` 0.23.43 | RUSTSEC-2026-0285/0346/0348/0349 | mittel | behoben: 0.23.46 (Patch) |
| `zip` 4.6.1 (über `tauri-plugin-updater`) | RUSTSEC-2026-0367/0368/0369 (Entpacken: SUID-Bits, Symlinks) | niedrig | **offen**: Fix erst in zip ≥ 9 (Major), nicht per Patch erreichbar; der Updater entpackt nur nach erfolgreicher Minisign-Prüfung, NSIS-Updates sind zudem keine ZIPs |
| `proc-macro-error`, `unic-*` | „unmaintained“-Warnungen (Build-Zeit, über Tauri) | info | offen, upstream |
| `source-map-js` 1.2.1 | GHSA-68fv-2mgg-jv7q (nur Build-Werkzeug) | niedrig | behoben: 1.2.2 |

## 4. Bekannte Einschränkungen

- **Linux:** keine Aufnahme (Portal/Wayland nicht umgesetzt). Bibliothek und Editor laufen.
- **macOS:** Bildschirmaufnahme-Freigabe nötig; bei ad-hoc-signierten Builds erlischt sie mit jedem Update (mit Developer-ID-Signatur aus der CI nicht mehr).
- **Windows:** „Fenster“ nimmt das Vordergrundfenster ohne Klick-Auswahl auf; der Ausschnitt gilt für den Monitor unter dem Mauszeiger (kein Ziehen über mehrere Monitore); Fensterschatten gibt es nur unter macOS.
- Löschen ist endgültig (kein Papierkorb).
- Bearbeitungen werden immer als PNG gespeichert.
- WebP wird nicht unterstützt.
- Dateinamen mit `:` in der Ablage werden ignoriert (unter Windows ohnehin unzulässig).
- Windows-Code wird auf dem Mac per `cargo clippy --target x86_64-pc-windows-msvc` mit Stub-Werkzeugen typgeprüft (kein Linken); gebaut und getestet wird er in der CI.
- Kein In-App-Changelog; Release-Notes stehen im Update-Dialog (aus den Commits).

## 5. Manuelle Test-Checkliste

### macOS (Apple Silicon und Intel)

- [ ] DMG öffnet ohne Gatekeeper-Warnung, App startet, Tutorial erscheint, Sprache = Systemsprache
- [ ] Erste Aufnahme fragt nach der Bildschirmaufnahme-Freigabe; nach Freigabe klappt die Aufnahme
- [ ] Ausschnitt / Fenster (mit und ohne Schatten) / Bildschirm per Button, Tray und Kürzel ⌘⇧7/8/9
- [ ] Esc bei Ausschnitt und Fenster: keine Fehlermeldung, keine Datei
- [ ] Verzögerung 3 s mit offenem Menü
- [ ] Editor öffnet nach der Aufnahme; alle Werkzeuge; ⌘Z/⌘⇧Z; Verpixeln + Speichern → Original in Vorschau prüfen
- [ ] „Als Kopie speichern“ → Filter „Bearbeitet“, Name „… (edit)“ bleibt nach Klick ins Namensfeld erhalten
- [ ] Zurück mit ungespeicherten Änderungen → Abfrage
- [ ] Kopieren → in Vorschau/Mail einfügen; Im Finder zeigen; Löschen
- [ ] Import PNG/JPG; Datei per Finder in den Ablage-Ordner kopieren → erscheint nach Neustart/Ordnerwechsel als Import
- [ ] Einstellungen: Kürzel auf etwas Belegtes (z. B. ⌘⇧3) → Meldung, alte Kürzel funktionieren weiter
- [ ] Sprache umstellen → Tray-Menü sofort umbenannt; Hell-Modus
- [ ] Autostart an → nach Ab-/Anmelden läuft screencap
- [ ] Fenster schließen → versteckt; Dock-Klick holt es zurück; Tray → Beenden
- [ ] Update-Prüfung in den Einstellungen (zeigt „aktuell“ bzw. 1.0.0)

### Windows 10/11 (100 % und 150 % Skalierung, möglichst zwei Monitore)

- [ ] Setup (NSIS) installiert, App startet ohne Konsolenfenster, Icon ohne weiße Ecken
- [ ] Ausschnitt: Overlay erscheint auf dem Monitor unter dem Mauszeiger, Größenanzeige stimmt, Ergebnis deckt sich pixelgenau mit der Auswahl (auch bei 150 %)
- [ ] Esc, Rechtsklick und Alt+F4 im Overlay brechen ab, Overlay verschwindet (kein Hänger, nächste Aufnahme sofort möglich)
- [ ] Fenster-Kürzel (Ctrl+Shift+8) ohne Vordergrundfenster (Desktop angeklickt) → screencap kommt nach vorn und meldet „Kein Fenster im Vordergrund“
- [ ] Kürzel zweimal schnell hintereinander → nur ein Overlay, eine Aufnahme
- [ ] Fenster: mit 3 s Verzögerung ein anderes Fenster anklicken → genau dieses Fenster im Bild, screencap selbst nicht
- [ ] Bildschirm auf dem zweiten Monitor (Mauszeiger dort)
- [ ] Kürzel Ctrl+Shift+7/8/9 aus anderen Apps; Tray-Symbol im Infobereich mit Menü
- [ ] Editor: Ctrl+Z/Ctrl+Y, Ctrl+D, Alt-Ziehen; Hinweistexte zeigen Ctrl/Alt
- [ ] „Im Explorer zeigen“ markiert die Datei; Kopieren → in Paint einfügen
- [ ] Bild in einem anderen Programm geöffnet halten und löschen → Fehlermeldung, Karte bleibt
- [ ] Ablage-Ordner auf anderes Laufwerk (z. B. `D:\Bilder\screencap`) und zurück
- [ ] Einstellungen: keine Fensterschatten-Option, Windows-Hinweistext; Autostart an/aus
- [ ] Fenster-X versteckt nur; Tray → Beenden beendet wirklich

## 6. Testlauf (lokal, 2026-10-19)

- `pnpm build` (tsc + vite) — fehlerfrei
- `cargo test --workspace` — 13 Core-Tests + 16 App-Tests grün
- `cargo clippy --workspace --all-targets` — eine bestehende Stil-Warnung (`needless_late_init`), keine Fehler
- `pnpm audit` — keine Befunde; `cargo audit` — nur die oben genannten offenen Upstream-Punkte
- UI-Durchlauf im Headless-Chrome mit IPC-Mocks: 38 + 8 + 7 Prüfungen (Bibliothek, Editor, Einstellungen, Hilfe, Windows-User-Agent, Auswahl-Overlay) grün; das Test-Gerüst lag nur im Scratch-Verzeichnis und ist nicht Teil des Repos
- Nicht lokal gebaut: Release-Bundles (`tauri build`) — kommen aus der CI (`workflow_dispatch`)

## 7. Verifikation (unabhängige Zweitprüfung, 2026-10-19)

Gegenprüfung des Stands `23e2dda` (Diff `1ffb614..23e2dda` vollständig gelesen),
alle Suiten selbst neu ausgeführt, danach eigene Korrekturen.

**Bestätigt:** Testzahlen (13 + 16), `pnpm build`, `pnpm audit` sauber, `cargo audit`
nur zip/unmaintained, Version 1.0.0 in `package.json`, `tauri.conf.json`,
`src-tauri/Cargo.toml`, `Cargo.lock` (Settings-Dialog liest `getVersion()`),
IPC-Namen/Argumente Frontend ↔ Rust, Pfad-Prüfung der Index-Einträge,
Dialog-Rechte (`confirm` läuft über `plugin:dialog|message`), CI-Workflow
unverändert tragfähig (Tests laufen dort auch auf Windows; keine Unix-Pfad-Annahmen).

**Widerlegt / korrigiert:**

- *Esc-Fix galt auch für Windows:* `capture_outcome` wertete jeden Fehler bei
  Ausschnitt/Fenster als Abbruch — unter Windows verschwanden damit echte Fehler
  („Kein Fenster im Vordergrund“, Overlay-/xcap-Fehler) kommentarlos. Jetzt nur
  unter macOS (dort beendet `screencapture` sich bei Esc mit Fehlercode); Windows
  meldet einen Abbruch ohnehin als `Ok` ohne Datei. Tests für beide Fälle.
- *Windows-Code „nicht lokal prüfbar“:* doch — `cargo xwin check/clippy --target
  x86_64-pc-windows-msvc --workspace --all-targets` lief mit Stub-Werkzeugen für
  `llvm-lib`/`llvm-rc` (nur Typprüfung, kein Linken) fehlerfrei durch.
- *Region-Capability „nur core:default“:* stimmt für Plugin-Rechte; eigene
  App-Befehle sind in Tauri 2 ohne App-Manifest für jedes Fenster erreichbar. Das
  Overlay lädt aber nur das eigene Bundle — kein zusätzliches Risiko, nur präzisiert.

**Zusätzlich behoben:**

- Gleichzeitige Aufnahmen (Kürzel zweimal, Tray + Kürzel): unter Windows
  zerstörte die zweite das Overlay der ersten, der erste Thread räumte danach den
  Rückkanal und das Overlay der zweiten ab → bis zu 180 s Hänger. Jetzt läuft
  immer nur eine Aufnahme (`AppState.capturing`).
- Overlay per Alt+F4 geschlossen → Aufnahme wartete 180 s; jetzt sofort Abbruch.
- Fehler bei Kürzel-/Tray-Aufnahmen landeten im Nichts; jetzt holt screencap das
  Fenster nach vorn und zeigt die Meldung (Event `capture-error`).
- Fehlermeldungen aus Rust (Löschen, Kürzel, Ablage-Ordner, Updates, Aufnahme)
  waren immer deutsch — jetzt in der eingestellten Sprache.
- Einstellungen: scheiterte das Schreiben von `settings.json`, blieben neue
  Kürzel/Autostart trotzdem aktiv — jetzt zurückgerollt.
- Clippy-Warnung `needless_late_init` behoben (0 Warnungen).
- Hilfe (DE/EN) um Alt+F4, „eine Aufnahme gleichzeitig“ und Fehleranzeige ergänzt.

**Testlauf nach den Korrekturen:** `cargo test --workspace` 13 + 17 grün,
`cargo clippy --workspace --all-targets` 0 Warnungen (macOS und Windows-Target),
`pnpm build` fehlerfrei. Die UI-Änderung (`capture-error`-Toast) ist nur per
Typprüfung abgesichert, nicht im Browser-Harness wiederholt.

**Weiterhin offen (bekannt):** Thumbnail-Cache-Schlüssel (Datei/Maße/Größe) kann
theoretisch bei byte-gleich großem Überschreiben veralten (nur bis Neustart);
JPG→PNG-Überschreiben unter Windows bei gesperrter JPG lässt die alte Datei
liegen (wird beim nächsten Abgleich als Import aufgenommen).

## 8. Runde 2 (2026-10-20)

Basis `5e927e3`. Abgleich mit den Fehlermustern L1–L13 aus Runde 1 der anderen
Apps, alle Treffer behoben. Neue Tests: Core 13 → 17, App 17 → 23; dazu ein
UI-Durchlauf im Headless-Chromium (20 Funktions- + 3 Windows- + 5 Layout-Szenarien,
Mock-IPC bildet das echte Dialog-Plugin nach, `window.confirm` ist wie in Tauri
ein Stub, der wirft).

| Muster | Befund in screencap | Status |
|---|---|---|
| L1 `window.confirm/alert` | nicht verwendet; Löschen/Verwerfen nutzen `confirm()` des Plugins — jetzt über `src/dialog.ts` mit Knöpfen in der UI-Sprache („Löschen“/„Abbrechen“, „Verwerfen“/„Abbrechen“), `kind: warning`; ein Dialogfehler zählt als „Nein“ | U |
| L2 Backend-Fehler roh | rohe `io`/`xcap`/`arboard`/Updater-Texte ohne Kontext (z. B. „Overlay: …“, „screencapture: …“, Lesefehler, Speichern, Zwischenablage, Ordner öffnen, Autostart) → mit Präfix in der UI-Sprache | A (Teil), S |
| L3 Erststart-Sprache | bereits `navigator.language`; Overlay-Hinweis war fest zweisprachig → folgt jetzt der Einstellung | U |
| L4 BOM | `index.json` mit UTF-8-BOM (Windows-Editor) galt als kaputt und wurde beiseitegelegt; `settings.json` mit BOM fiel still auf Standardwerte zurück → BOM wird toleriert | A |
| L5 Windows-Dateisystem | Rename (Bild, Index, Settings, Thumbnail) und Löschen wiederholen bei Sperrfehlern 5/32/33 (~1,5 s); `Shot.PNG` überschreiben wurde wegen Groß-/Kleinschreibung zu `Shot-2.png`; Import übernahm `: * ? " < > |` und Gerätenamen (`CON.png`) → portabel bereinigt; Index-Namen werden unter Windows zusätzlich auf diese Zeichen, Steuerzeichen, Punkt am Ende und Gerätenamen geprüft; Ablage-Ordner `D:` → `D:\`, `~` wird aufgelöst, relative Pfade abgelehnt | A |
| L6 Strg/⌘ | deutsche Windows-Hinweise und Hilfe sagten „Ctrl“ → „Strg“; Kürzel-Schreibweise `CmdOrCtrl` erklärt | U (Windows-UA) |
| L7 Remote-Inhalte | keine (Update-Notizen als Text, CSP ohne Fremdquellen) | ok |
| L8 Hauptthread | alle schweren Befehle (`get_image`, `get_thumb`, `save_edit`, `import_files`, `copy_shot`, `delete_shot`, `set_settings` inkl. Ordnerwechsel, `sync_library`, Index-Schreiben) waren synchrone Tauri-Befehle = Hauptthread → jetzt `async` + `spawn_blocking`; Sync erzeugt keine Thumbnails mehr (lazy, eins nach dem anderen); `save_edit` decodiert das PNG nur noch einmal | A (Logik), S (Threading) |
| L9 Laufende Aufgaben | Doppelklick „Als Kopie speichern“ legte zwei Kopien an; Speichern + sofort „Zurück“ öffnete den Editor wieder; zweimal „Installieren“ (Banner + Dialog) startete zwei Downloads; Ordnerwechsel während einer Aufnahme legte den Index-Eintrag im falschen Ordner an → alle gesperrt | U, S |
| L10 Zeit | Editor-Datum in Systemsprache statt UI-Sprache; `index.corrupt-<Zeit>` in UTC → lokale Zeit | U |
| L11 Fehler sichtbar | Umbenennen/Tags/Index-Schreibfehler/Bild-Ladefehler/Update-Installation/Zwischenablage nach Aufnahme wurden verschluckt; Fehler-Toasts verschwanden nach 1,5 s → Fehler rot, 8 s, per Klick schließbar | U |
| L12 Texte | Details-Labels `px/file/size/date`, Dateifilter „Images“, Overlay-Hinweis fest → i18n; Hilfe DE/EN um alle Änderungen ergänzt; Esc schließt Hilfe und Einstellungen (Hilfe fängt Esc/Entf ab, statt dass der Editor zur Bibliothek springt bzw. ein Objekt löscht) | U |
| L13 Updater | Fehler der Installation wurden verschluckt, Prüf-Fehler ohne Grund → Grund in der UI-Sprache; Pubkey/HTTPS-Endpoint unverändert; Vergleich übernimmt der Updater (SemVer, 1.0.0) | U, M |

**Weitere Funde:**

- Standard-Verzögerung mit krummem Wert (z. B. 7 s): das Dropdown zeigte „sofort“, aufgenommen wurde mit 7 s → Wert erscheint als eigener Eintrag.
- Update-Banner schwebte über der Editor-Werkzeugleiste und dem Einstellungen-Dialog (bei 860 px Breite verdeckte es Werkzeuge) → im Fluss unter der Kopfzeile.
- „Wählen …“ brach im Einstellungen-Dialog um.
- JPG→PNG-Überschreiben bei kurz gesperrter JPG: Löschen wird jetzt wiederholt (Rest-Risiko bei dauerhaft geöffneter Datei bleibt, siehe unten).

**Layout geprüft** bei 860×560 (Mindestgröße), DE/EN × Dunkel/Hell: Bibliothek,
langer Fehler-Toast, Editor, Hilfe, Einstellungen, Auswahl-Overlay — kein
horizontaler Überlauf, keine abgeschnittenen Knöpfe (Screenshots gesichtet).

**Testlauf:** `cargo test --workspace` 17 + 23 grün; `cargo clippy --workspace
--all-targets` 0 Warnungen auf macOS und für `x86_64-pc-windows-msvc`; `pnpm build`
fehlerfrei; `pnpm audit` ohne Befund; `cargo audit` unverändert (zip 4.6.1 über den
Updater — auch `tauri-plugin-updater` 2.14 hängt noch an zip 4, kein Update möglich —
sowie Unmaintained-Hinweise, neu `zbus_names`/`zvariant` nur Linux).

**Nur auf echter Hardware prüfbar (zusätzlich zur Checkliste in Abschnitt 5):**

- [ ] Windows: Bild in der Windows-Fotoanzeige offen lassen, im Editor „Speichern“ → klappt nach kurzer Wiederholung bzw. meldet den Fehler
- [ ] Windows: Löschen einer kurz gesperrten Datei (OneDrive-Ordner) → gelingt ohne Meldung
- [ ] Beide: Dialoge zeigen „Löschen“/„Abbrechen“ bzw. „Verwerfen“/„Abbrechen“, Esc = Abbrechen (Windows: TaskDialog)
- [ ] Beide: Editor mit 5K-Aufnahme öffnen/speichern — Fenster bleibt bedienbar (kein „Keine Rückmeldung“ / Beachball)
- [ ] Beide: Ablage-Ordner mit vielen fremden Bildern wählen → Wechsel sofort, Thumbnails erscheinen nach und nach
- [ ] Beide: Ablage-Ordner schreibgeschützt → Tag setzen meldet „Bibliothek nicht gespeichert“
- [ ] Windows: Ablage-Ordner „D:“ eintippen → wird zu `D:\`, Aufnahmen landen dort
- [ ] Update-Installation mit abgebrochenem Download → Meldung mit Grund, Knopf wieder aktiv

**Weiterhin offen:** zip-Advisories (upstream); dauerhaft gesperrte JPG beim
JPG→PNG-Überschreiben bleibt liegen und wird beim nächsten Abgleich als Import
aufgenommen; Thumbnail-Cache-Schlüssel (siehe Abschnitt 7).

## 9. Runde 2 – Verifikation (unabhängige Zweitprüfung, 2026-10-20)

Basis `0944054`. Diff `5e927e3..0944054` gegengelesen (Schwerpunkt: neue
Nebenläufigkeit durch `async` + `spawn_blocking`, Windows-Pfade), alle Tests und
Typchecks neu ausgeführt, die UI-Szenarien erneut und um eigene ergänzt.

**Bestätigt:** BOM-Toleranz, Windows-Namensregeln, Import-Bereinigung,
`D:`/`~`/relative Ablage-Ordner, Groß-/Kleinschreibung beim Überschreiben,
lokalisierte Dialogknöpfe (Abbrechen löscht nicht), Fehler-Toasts, Esc in
Hilfe/Einstellungen, Doppelklick-Sperre, Strg in DE-Windows-Texten. Die
Global-Shortcut-Registrierung aus dem Worker-Thread ist unbedenklich (das Plugin
leitet sie selbst auf den Hauptthread um), ebenso Tray-Texte.

**Widerlegt / neu behoben:**

| Befund | Folge | Behebung |
|---|---|---|
| Durch `async` liefen Schreibbefehle (Umbenennen, Tags, Favorit, Löschen, Speichern, Import, Abgleich) jetzt **parallel** zum Ordnerwechsel. `set_settings` stellt erst die Einstellungen auf den neuen Ordner und lädt danach dessen Index — ein Schreibvorgang dazwischen schrieb die **alte** Bibliothek in die `index.json` des **neuen** Ordners (Namen/Tags dort verloren); ein laufender Import legte Einträge für Dateien an, die im alten Ordner lagen. Vorher serialisierte der Hauptthread das. | Datenverlust (selten, Zeitfenster) | `AppState::io`: ein Schreib-Lock für alle Bibliotheks-Schreibbefehle und den kompletten Ordnerwechsel. Lesen (`list_shots`, `get_image`, `get_thumb`) bleibt frei. |
| Ordnerwechsel vs. Aufnahme: die Prüfung „läuft eine Aufnahme?“ und der Start einer Aufnahme konnten sich überholen (Prüfung negativ, Aufnahme startet, Ziel-Ordner noch der alte). | Aufnahme im falschen Ordner | `do_capture` setzt das Aufnahme-Flag und bestimmt den Ziel-Ordner unter demselben Lock; während der eigentlichen Aufnahme (Auswahl ziehen) ist es nicht gehalten, beim Ablegen wieder. |
| `persist()` hielt die Bibliotheks-Sperre während des Schreibens samt Windows-Sperr-Wiederholungen; `list_shots` läuft synchron auf dem Hauptthread und wartete mit. | UI-Hänger bis ~2,7 s bei gesperrter `index.json` (OneDrive) | Index wird als Kopie geschrieben (Reihenfolge sichert der Schreib-Lock). |
| `retry_io` wartete laut Doku „~1,5 s“, tatsächlich 2,7 s. | Längere Wartezeit als dokumentiert | 40, 80 … 320 ms = 1,44 s. |
| Editor: Speichern/Löschen läuft, eine neue Aufnahme öffnet den Editor (Kürzel) → das späte Ergebnis sprang zur alten Aufnahme zurück und setzte den Änderungsstand der neuen auf „gespeichert“ (Verlassen fragte nicht mehr). Im Browser mit dem alten Code nachgestellt. | Ungespeicherte Änderungen still verworfen | Ergebnis wird nur angewandt, wenn noch dieselbe Aufnahme offen ist. |
| Update-Installationsfehler zeigten den Präfix doppelt („Update fehlgeschlagen: Update fehlgeschlagen: …“) in Banner und Einstellungen — die Runde-2-Szenarien prüften nur `includes`. Im Browser mit dem alten Code nachgestellt. | Kosmetik (L13) | Frontend zeigt die Backend-Meldung unverändert (trägt den Präfix schon). |
| Leerer `impl Settings {}`-Block | – | entfernt |

**Stichproben L1–L13 an nicht genannten Stellen:** kein `window.confirm/alert/prompt`
im Code; EN-Texte sagen weiter „Ctrl“, DE „Strg“; Undo/Redo-Tooltips ohne
Kürzel; Region-Fenster darf `get_settings` aufrufen (App-Befehle ohne
App-Manifest sind für alle Fenster erlaubt); Updater-Endpoint HTTPS, Pubkey gesetzt;
verbleibende rohe `to_string()`-Fehler nur für interne Fälle (JoinError,
Thumbnail-Lesefehler — die Karte bleibt dann ohne Vorschau, bewusst ohne Meldung pro Karte).

**Testlauf:** `cargo test --workspace` 17 + 23 grün; `cargo clippy --workspace
--all-targets` 0 Warnungen auf macOS und für `x86_64-pc-windows-msvc` (nur
Typcheck, nichts gelinkt/ausgeführt); `pnpm build` fehlerfrei; `pnpm audit` ohne
Befund; `cargo audit` wie in Abschnitt 8 (zip 4.6.1 upstream). UI im
Headless-Chromium: 20/20 + 3/3 Windows-UA + 5/5 Layout (860×560) + 4/4 neue
Szenarien (Speichern bzw. Löschen läuft während eine neue Aufnahme den Editor
öffnet; Installationsfehler-Präfix in Banner und Einstellungen). Der neue
Schreib-Lock ist nur gegengelesen und typgeprüft — echte Parallelität (Tauri-
Runtime) lief hier nicht.

**Zusätzlich nur auf echter Hardware prüfbar:**

- [ ] Beide: großen Import (100+ Bilder) starten und sofort den Ablage-Ordner wechseln → Wechsel wartet, danach stimmen beide Ordner (Index-Einträge nur für vorhandene Dateien)
- [ ] Beide: Editor mit Änderungen speichern und währenddessen per Kürzel aufnehmen (Editor-nach-Aufnahme an) → bleibt bei der neuen Aufnahme
- [ ] Windows: `index.json` in OneDrive-Ordner, während des Syncs Tags setzen → Bibliothek bleibt bedienbar

## 10. Runde 3 (2026-10-20)

Basis `712f76f`. Schwerpunkte: jede Aussage aus Hilfe (DE/EN), README und
In-App-Texten gegen den Code geprüft; End-to-End-Abläufe inkl. Fehlerfällen;
die in Runde 2 nur typgeprüfte Schreib-Sperre mit echten Threads getestet;
die beiden offenen Restpunkte (gesperrte JPG, Thumbnail-Cache) behoben.

**Falsche oder ungenaue Aussagen**

| Aussage | Befund | Lösung |
|---|---|---|
| Hilfe/Tutorial: „Dateien, die du selbst hineinlegst, übernimmt screencap **automatisch**; gelöschte verschwinden“ | Abgleich lief nur beim Start und beim Ordnerwechsel — eine im Finder/Explorer kopierte Datei erschien erst nach einem Neustart (`sync_library` wurde vom Frontend nie aufgerufen) | Code: das Hauptfenster gleicht jetzt bei jedem Fokus ab (höchstens alle 2 s); `index.json` wird nur bei einer Änderung geschrieben (kein Plattenzugriff, keine wiederholte Fehlermeldung bei schreibgeschütztem Ordner); während einer Aufnahme wird nicht abgeglichen (die fertige Datei läge sonst kurz vor dem Einsortieren als „Import“ doppelt in der Bibliothek). Hilfe DE/EN nennt jetzt die Zeitpunkte. |
| Hilfe: „Bildschirm — der komplette Monitor“ | macOS: `screencapture` legt bei einer Zieldatei nur den Hauptbildschirm ab („1 file per screen“), Windows: Monitor unter dem Mauszeiger | Code: `-m` macht die Wahl unter macOS ausdrücklich; Hilfe DE/EN nennt beide Fälle |
| Hilfe: „Verzögerung wartet … vor der Aufnahme“ | Windows-Ausschnitt: erst Bereich ziehen, dann Verzögerung | Hilfe DE/EN ergänzt |
| Hilfe: „Standard-Ablage ~/Pictures/screencap“ | Windows: Bilder-Ordner des Benutzers (`dirs::picture_dir`) | Hilfe DE/EN ergänzt |
| Hilfe: JPG wird beim Überschreiben zur PNG | stimmte, aber bei gesperrter JPG blieb sie liegen (siehe unten) | Verhalten geändert, Hilfe DE/EN beschreibt es |

Geprüft und zutreffend (Code gelesen, teils Test/Harness): 6 Farben, Stärken
2/4/8 px, Pfeiltasten 1/10 px, ⌘D/⌘C/⌘V/⌥-Ziehen, Undo/Redo inkl. Strg+Y, Esc-Kaskade
im Editor (Zuschnitt → Auswahl → Bibliothek), Verzögerungen 0/3/5/10 + krummer
Standardwert, Pixelgröße im Overlay, Alt+F4/Rechtsklick/Esc, eine Aufnahme
gleichzeitig, Import nur PNG/JPG mit Inhaltsprüfung und Namensbereinigung,
Wiederholen bei Sperren (1,44 s), `index.corrupt-<Datum>.json`, BOM, Update-Prüfung
beim Start ohne Auto-Installation, Changelog im Einstellungen-Dialog, Kürzel-Rollback,
Tray-Sprache, Esc schließt Einstellungen ohne Speichern. DE- und EN-Hilfe haben
dieselbe Gliederung (16 Abschnitte/Schritte, gleiche Absatzzahl je Abschnitt).

**Restpunkte aus Runde 1/2 — behoben**

- *JPG→PNG-Überschreiben bei dauerhaft gesperrter JPG (Windows):* bisher blieb
  die JPG neben der neuen PNG liegen und wurde beim nächsten Abgleich als zweite,
  unbearbeitete „Import“-Aufnahme übernommen. Jetzt (`replace_with_png`): lässt sich
  die JPG nach den Wiederholungen nicht entfernen, wird die neue PNG wieder
  gelöscht und das Speichern schlägt mit klarer Meldung fehl; die Änderungen bleiben
  im Editor („Programm schließen und erneut speichern — oder Als Kopie speichern“).
- *Thumbnail-Cache-Schlüssel:* jede Aufnahme trägt jetzt `rev` (zählt bei jedem
  „Speichern“ hoch, fehlt in alten `index.json` → 0); der Cache-Schlüssel der
  Galerie enthält `rev`, auch ein Überschreiben mit gleicher Größe und gleichen
  Maßen liefert also ein neues Thumbnail. Zusätzlich erzeugt `get_thumb` ein
  Thumbnail neu, wenn das Bild neuer ist als das Thumbnail (Datei im
  Finder/Explorer unter gleichem Namen ersetzt).

**Schreib-Sperre: echte Nebenläufigkeitstests** (`src-tauri/src/concurrency_tests.rs`)

Die Bibliotheks-Operationen sind dafür aus den Tauri-Befehlen herausgelöst
(`rename_op`, `set_tags_op`, `delete_op`, `save_edit_op`, `import_op`, `sync_op`,
`file_capture`, `load_current_library` — sie nehmen `&AppState`; die Befehle sind
nur noch Hüllen mit `spawn_blocking` + Event). Die Tests laufen mit echten
OS-Threads gegen echte Ordner und lesen danach `index.json` von der Platte (=
Neustart):

- Import von 40 Bildern + gleichzeitiger Ordnerwechsel (5 Durchläufe): beide
  Ordner-Indizes listen genau ihre Dateien, Bibliothek im Speicher = Index des
  aktiven Ordners.
- 25× „Speichern“ (Überschreiben) parallel zu 25 einsortierten Aufnahmen: keine
  verlorenen Einträge, letzter Stand gewinnt, `rev` = 25.
- 50× Umbenennen + Tags parallel zu 10 Abgleichen mit 60 fremden Dateien: Name und
  Tags bleiben, keine doppelten Einträge.
- Abgleich während einer Aufnahme übernimmt die neue Datei nicht doppelt.
- Gesperrte JPG (simuliert über eine fehlschlagende Löschfunktion) bleibt
  unverändert, keine verwaiste PNG; JPG→PNG über `save_edit_op` mit `rev`.
- Schreibgeschützter Ordner (Unix): Tags und „Als Kopie“ melden einen Fehler mit
  Dateiname.

Gegenprobe: mit ausgehebelter Sperre schlagen 2 der 3 Lock-Tests fehl (Import +
Ordnerwechsel, Umbenennen + Abgleich); der Speichern-+-Aufnahme-Test bleibt dann grün
(das Zeitfenster ist dort zu klein) — er sichert nur das Ergebnis, nicht die Sperre.
Die Tests laufen ohne Tauri-Laufzeit; `spawn_blocking` selbst ist nicht beteiligt.

**End-to-End-Abläufe** (Headless-Chromium, echte React-Oberfläche, Mock-IPC wie in
Runde 2, Dialog-Plugin nachgebildet), je in DE und EN = 24/24:

1. Erststart: Systemsprache übernommen, Tutorial alle 6 Schritte, danach nicht wieder
2. Aufnahme → Editor öffnet → Rechteck → Speichern → `rev` +1 → Galerie holt neues Thumbnail
3. Aufnahme abgebrochen (Esc): keine Karte, keine Meldung
4. Platte voll beim Speichern: rote Meldung, Änderungen bleiben, Zurück fragt nach
5. Gesperrte JPG beim Überschreiben: Meldung, Editor bleibt, „Als Kopie speichern“ klappt
6. Import-Dialog abgebrochen: kein Import, keine Meldung
7. Import ohne gültige Bilder: Hinweis „Keine importierbaren Bilder“
8. Ablage-Ordner schreibgeschützt: Meldung im Dialog, Dialog bleibt offen
9. Datei im Finder/Explorer hineinkopiert/gelöscht: erscheint/verschwindet beim Fokus, 2-s-Drossel
10. Neustart mitten in der Bearbeitung: Bibliothek lädt sauber, kein Fehler
11. Einstellungen per Esc: nichts gespeichert

Die Fehlerfälle „Platte voll“/„schreibgeschützt“/„gesperrt“ sind in der
Oberfläche über die Fehlermeldungen simuliert, die das Backend liefert; im Backend
selbst ist „schreibgeschützt“ echt getestet, „Platte voll“ und echte
Windows-Sperren nicht. Ältere Suiten erneut grün: 20/20, 4/4 Verifikation, 3/3
Windows-UA.

**Testlauf:** `cargo test --workspace` 18 Core + 32 App grün; `cargo clippy
--workspace --all-targets` 0 Warnungen auf macOS und für `x86_64-pc-windows-msvc`
(nur Typcheck); `pnpm build` fehlerfrei; `pnpm audit` ohne Befund; `cargo audit`
wie in Runde 2 (zip 4.6.1 über den Updater; Unmaintained-/Linux-Hinweise inkl.
`glib`, nur Linux).

**Nur auf echter Hardware prüfbar (zusätzlich):**

- [ ] Beide: Datei im Finder/Explorer in den Ablage-Ordner kopieren, zurück zu screencap klicken → Karte erscheint ohne Neustart; Datei löschen → verschwindet
- [ ] Beide: Aufnahme per Kürzel bei geöffnetem Fenster → genau eine Karte (kein zusätzlicher „Import“)
- [ ] macOS mit zwei Monitoren: „Bildschirm“ nimmt den Hauptbildschirm auf
- [ ] Windows: JPG-Aufnahme in der Fotoanzeige geöffnet lassen, im Editor „Speichern“ → Meldung „gesperrt“, keine zweite Datei im Ordner; „Als Kopie speichern“ klappt
- [ ] Beide: Aufnahme zweimal mit gleicher Größe speichern → Galerie zeigt das neue Bild

**Weiterhin offen:** zip-Advisories (upstream). Ein Bild mit Änderungsdatum in der
Zukunft bekommt bei jeder ersten Anzeige pro Sitzung ein neues Thumbnail (nur
Rechenzeit).
