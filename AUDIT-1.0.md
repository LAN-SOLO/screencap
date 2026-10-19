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
- `cargo check --target x86_64-pc-windows-msvc` lässt sich auf dem Mac nicht ausführen (C-Build von `ring`); der Windows-Code ist statisch geprüft und wird in der CI gebaut.
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
