# screencap. — Entwicklungsplan

Screenshot-Tool mit Ablage: Aufnahme, Anmerkungs-Editor, durchsuchbare Bibliothek (Tauri 2 + React).
Website: https://lan-solo.de/de/tools/screencap

## Architektur

- `core/` (`screencap-core`) — reine Rust-Logik: Bibliotheks-Index (`index.json`
  im Ablage-Ordner), Suche/Tags/Favoriten, Disk-Sync. Unit-getestet.
- `src-tauri/` — App-Schicht: Aufnahme über natives macOS `screencapture`
  (Ausschnitt `-i`, Fenster `-i -W`, Bildschirm; Verzögerung `-T`),
  Thumbnails, Tray, globale Shortcuts, Commands, Updater.
- `src/` — React-UI: Galerie mit Filtern/Suche/Tags, Canvas-Editor
  (Stift, Linie, Pfeil, Rechteck, Ellipse, Text, Verpixeln, Zuschneiden,
  Undo/Redo), Einstellungen. i18n DE/EN, dunkles LAN-SOLO-Theme.

## Datenablage

- Screenshots: normale Bilddateien in `~/Pictures/screencap/` (Ordner wählbar).
- Index: `index.json` daneben — Namen, Tags, Favoriten, Aufnahmeart.
- Thumbnails: `.thumbs/<id>.png` (max. 512 px), werden bei Bedarf neu erzeugt.
- Dateien, die außerhalb der App in den Ordner gelangen, werden beim Sync
  automatisch als Import übernommen; gelöschte verschwinden aus dem Index.

## Phasen

- [x] **Phase 0 — Gerüst:** Tauri-2-Workspace nach keypile-Vorbild, Icons aus
  Website-SVG, Updater-Schlüsselpaar (`~/.tauri/screencap-updater.key`).
- [x] **Phase 1 — Core:** Library-Index mit Suche (Name/Datei/Tags), Filtern
  (Favoriten/Art), Tag-Normalisierung, Disk-Sync. 8 Unit-Tests.
- [x] **Phase 2 — Aufnahme:** `screencapture`-Pipeline (Ausschnitt/Fenster/
  Bildschirm, Verzögerung, Fensterschatten, eigenes Fenster wird versteckt),
  Ablage + Thumbnail + optional Zwischenablage/Editor.
- [x] **Phase 3 — Bibliothek-UI:** Galerie-Grid, Filter-Chips, Suche, Karten
  mit Hover-Aktionen (Kopieren/Favorit/Finder/Löschen), Import-Dialog.
- [x] **Phase 4 — Editor:** Canvas-Editor mit Stift/Linie/Pfeil/Rechteck/
  Ellipse/Text/Verpixeln/Zuschneiden, 6 Farben, 3 Strichstärken, Undo/Redo,
  Speichern (Überschreiben oder Kopie), Metadaten-Panel (Name/Tags/Favorit).
- [x] **Phase 5 — System-Integration:** Tray mit Aufnahme-Menü, globale
  Shortcuts (Cmd+Shift+7/8/9), Autostart, Schließen = Verstecken, Updater.
- [ ] **Phase 6 — später:** Windows/Linux-Aufnahme, OCR (screencap framed),
  Quell-App/Fenstertitel-Metadaten, Schnellaktionen, Scroll-Capture.

## Shortcuts (Standard)

- `Cmd/Ctrl+Shift+7` — Ausschnitt · `…+8` — Fenster · `…+9` — Bildschirm

## Build

- `pnpm install` · `pnpm tauri dev`
- Release: `TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/screencap-updater.key)" \
  TAURI_SIGNING_PRIVATE_KEY_PASSWORD="" pnpm tauri build --bundles app,dmg`

## Hinweis macOS

Erste Aufnahme fragt nach der Bildschirmaufnahme-Freigabe
(Systemeinstellungen → Datenschutz & Sicherheit → Bildschirmaufnahme).
