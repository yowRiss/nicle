import { invoke } from '@tauri-apps/api/core';

export type Entry = { readonly name: string; readonly path: string; readonly directory: boolean };
export type Page = { readonly entries: Entry[]; readonly more: boolean };
export type FolderDialogResult =
  | { type: 'Selected'; path: string }
  | { type: 'Cancelled' }
  | { type: 'NotSupported' };

export const api = {
  open: (path: string) => invoke<string>('open_workspace', { path }),
  chooseFolder: (defaultPath?: string) =>
    invoke<FolderDialogResult>('choose_folder', { defaultPath }),
  revealInFileManager: (path: string) =>
    invoke<void>('reveal_in_file_manager', { path }),
  list: (path: string, offset = 0) => invoke<Page>('list_directory', { path, offset }),
  read: (path: string) => invoke<string>('read_file', { path }),
  save: (path: string, content: string) => invoke<void>('save_file', { path, content }),
  create: (path: string, directory: boolean) => invoke<void>('create_entry', { path, directory }),
  rename: (path: string, destination: string) => invoke<void>('rename_entry', { path, destination }),
  delete: (path: string) => invoke<void>('delete_entry', { path }),
  search: (query: string) => invoke<string[]>('search_files', { query }),
  cancelSearch: () => invoke<void>('cancel_search'),
};

export const basename = (path: string) => path.split(/[\\/]/).pop() ?? path;
export const parent = (path: string) => path.includes('/') ? path.slice(0, path.lastIndexOf('/')) : '';
export const errorText = (error: unknown) => error instanceof Error ? error.message : String(error);
