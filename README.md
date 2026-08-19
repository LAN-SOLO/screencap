# screencap.

Screenshot-Tool mit Ablage: aufnehmen, markieren, wiederfinden.

- **Aufnahme:** Ausschnitt, Fenster oder ganzer Bildschirm — per Button,
  Tray oder Kürzel (`Cmd+Shift+7/8/9`), mit Verzögerung.
- **Editor:** Pfeile, Formen, Text, Stift, Verpixeln (endgültig, keine
  abnehmbare Ebene), Zuschneiden, Undo/Redo — direkt nach der Aufnahme.
- **Bibliothek:** alles landet durchsuchbar in `~/Pictures/screencap/` —
  mit Namen, Tags, Favoriten und Thumbnails. Ordner frei wählbar, Dateien
  bleiben normale PNGs/JPGs.
- Tray, Autostart, DE/EN, signierte In-App-Updates. Alles lokal.

## Entwicklung

```sh
pnpm install
pnpm tauri dev              # App im Dev-Modus
cargo test -p screencap-core   # Core-Tests
```

Release-Build (macOS, mit Updater-Artefakten):

```sh
TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/screencap-updater.key)" \
TAURI_SIGNING_PRIVATE_KEY_PASSWORD="" \
pnpm tauri build --bundles app,dmg
```

Details: `SCREENCAP_PLAN.md`.
