import { invoke } from '@tauri-apps/api/core';

export type ShotKind = 'region' | 'window' | 'screen' | 'import' | 'edit';
export type Filter = 'all' | 'favorites' | 'region' | 'window' | 'screen' | 'edits';

export interface Shot {
  id: string;
  file: string;
  name: string;
  kind: ShotKind;
  capturedAt: string;
  width: number;
  height: number;
  sizeBytes: number;
  tags: string[];
  favorite: boolean;
}

export interface Settings {
  language: 'de' | 'en';
  libraryDir: string;
  format: 'png' | 'jpg';
  delayDefault: number;
  copyAfterCapture: boolean;
  openEditorAfter: boolean;
  windowShadow: boolean;
  autostart: boolean;
  shortcutRegion: string;
  shortcutWindow: string;
  shortcutScreen: string;
}

export interface UpdateInfo {
  version: string;
  notes: string | null;
  date: string | null;
}

export const api = {
  capture: (kind: 'region' | 'window' | 'screen', delay?: number) =>
    invoke<string | null>('capture', { kind, delay: delay ?? null }),
  listShots: (query: string, filter: Filter) =>
    invoke<Shot[]>('list_shots', { query, filter }),
  getThumb: (id: string) => invoke<string>('get_thumb', { id }),
  getImage: (id: string) => invoke<string>('get_image', { id }),
  renameShot: (id: string, name: string) => invoke<void>('rename_shot', { id, name }),
  setTags: (id: string, tags: string[]) => invoke<void>('set_tags', { id, tags }),
  setFavorite: (id: string, favorite: boolean) =>
    invoke<void>('set_favorite', { id, favorite }),
  deleteShot: (id: string) => invoke<void>('delete_shot', { id }),
  revealShot: (id: string) => invoke<void>('reveal_shot', { id }),
  copyShot: (id: string) => invoke<void>('copy_shot', { id }),
  saveEdit: (id: string, pngBase64: string, mode: 'overwrite' | 'copy') =>
    invoke<Shot>('save_edit', { id, pngBase64, mode }),
  importFiles: (paths: string[]) => invoke<number>('import_files', { paths }),
  syncLibrary: () => invoke<void>('sync_library'),
  libraryPath: () => invoke<string>('library_path'),
  getSettings: () => invoke<Settings>('get_settings'),
  setSettings: (s: Settings) => invoke<void>('set_settings', { new: s }),
  checkUpdate: () => invoke<UpdateInfo | null>('check_update'),
  installUpdate: () => invoke<void>('install_update'),
};
