# screencap.

Screenshot-Tool mit Ablage: aufnehmen, markieren, wiederfinden.

- **Aufnahme:** Ausschnitt, Fenster oder ganzer Bildschirm — per Button,
  Tray oder Kürzel (`Cmd/Ctrl+Shift+7/8/9`), mit Verzögerung. macOS über
  `screencapture`, Windows über `xcap` + eigenes Auswahl-Overlay; Linux noch
  ohne Aufnahme.
- **Editor:** Pfeile, Formen, Text, Stift, Verpixeln (endgültig, keine
  abnehmbare Ebene), Zuschneiden, Undo/Redo (auch `Cmd/Ctrl+Z`) — direkt
  nach der Aufnahme; Objekte bleiben bis zum Speichern editierbar.
- **Bibliothek:** alles landet durchsuchbar in `~/Pictures/screencap/` —
  mit Namen, Tags, Favoriten und Thumbnails. Ordner frei wählbar, Dateien
  bleiben normale PNGs/JPGs (Import: PNG/JPG).
- Tray, Autostart, DE/EN, signierte In-App-Updates. Alles lokal.
- Windows-fest: Dateinamen beim Import portabel bereinigt, Speichern/Löschen
  wiederholen kurz bei gesperrten Dateien (Virenscanner, OneDrive), schwere
  Arbeit (Bilder laden/speichern, Import, Ordnerwechsel) läuft nicht im
  UI-Thread; Schreibzugriffe auf die Bibliothek laufen nacheinander, damit ein
  Ordnerwechsel nie mit Import/Speichern/Aufnahme kollidiert.

## Entwicklung

```sh
pnpm install
pnpm tauri dev              # App im Dev-Modus
cargo test --workspace      # Core- und App-Tests (vorher einmal pnpm build)
```

Version 1.0.0 — Audit-Bericht (inkl. Runde 2 und Verifikation), Sicherheitsbefunde und manuelle
Test-Checkliste (macOS/Windows): `AUDIT-1.0.md`.

Release-Build (macOS, mit Updater-Artefakten):

```sh
TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/screencap-updater.key)" \
TAURI_SIGNING_PRIVATE_KEY_PASSWORD="" \
pnpm tauri build --bundles app,dmg
```

Details: `SCREENCAP_PLAN.md`.
