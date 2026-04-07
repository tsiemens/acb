import { reactive, ref } from 'vue';
import { parse_config, serialize_config } from '../pkg/acb_wasm.js';
import { FileKind, getFileManagerStore, modifyDrawerNotificationForUserAddedFiles, type FileEntry } from './file_manager_store.js';

// -- Config types (mirrors Rust AcbConfig / AccountBindings) --

export interface AccountBindings {
   questrade: Record<string, string>;
   rbc_di: Record<string, string>;
   etrade: Record<string, string>;
}

export interface AcbConfig {
   version: number;
   account_bindings: AccountBindings;
}

export interface ConfigParseResult {
   config: AcbConfig;
   warnings: string[];
}

// -- localStorage persistence --

const STORAGE_KEY = 'acb_config';
const DEFAULT_CONFIG_FILE_NAME = 'acb-config.json';

/** Reactive flag tracking whether a config exists in localStorage. */
export const configExists = ref(localStorage.getItem(STORAGE_KEY) !== null);

function loadConfigFromStorage(): AcbConfig | undefined {
   try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (!raw) return undefined;
      // Re-validate through WASM to ensure it's still valid.
      const result = parseConfigJson(raw);
      return result.config;
   } catch (e) {
      console.warn('Failed to load config from localStorage:', e);
      return undefined;
   }
}

function saveConfigToStorage(config: AcbConfig): void {
   try {
      const json = serializeConfig(config);
      localStorage.setItem(STORAGE_KEY, json);
      configExists.value = true;
   } catch (e) {
      console.warn('Failed to save config to localStorage:', e);
   }
}

// -- WASM wrappers --

/** Parse a JSON string into a validated AcbConfig via WASM.
 *  Throws on invalid JSON or unsupported version. */
export function parseConfigJson(json: string): ConfigParseResult {
   // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
   const result = parse_config(json);
   return {
      // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
      config: result.config as AcbConfig,
      // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
      warnings: (result.warnings ?? []) as string[],
   };
}

/** Serialize an AcbConfig to pretty-printed JSON via WASM. */
export function serializeConfig(config: AcbConfig): string {
   return serialize_config(config);
}

// -- Store --

export interface ConfigStore {
   /** The current config, or null if none loaded. */
   config: AcbConfig | null;
   /** The file entry ID in the file manager for the config file, or null. */
   fileEntryId: number | null;
}

let _store: ConfigStore | null = null;

export function getConfigStore(): ConfigStore {
   if (!_store) {
      const initial = loadConfigFromStorage() ?? null;
      _store = reactive({
         config: initial,
         fileEntryId: null,
      }) as ConfigStore;

      // If a config was loaded from storage, sync it to the file drawer.
      if (initial) {
         syncConfigToFileDrawer(_store);
      }
   }
   return _store;
}

/** Returns the current config JSON string for passing to WASM convert
 *  functions, or undefined if no config is loaded. */
export function getConfigJsonForWasm(): string | undefined {
   const store = getConfigStore();
   if (!store.config) return undefined;
   try {
      return serializeConfig(store.config);
   } catch {
      return undefined;
   }
}

/**
 * Set a new config, persisting to localStorage and updating the file drawer.
 */
export function setConfig(store: ConfigStore, config: AcbConfig): void {
   store.config = config;
   saveConfigToStorage(config);
   syncConfigToFileDrawer(store);
}

/**
 * Parse and load a config from a JSON string (e.g. from an uploaded file).
 * Returns warnings from validation.  Throws on parse errors.
 */
export function loadConfigFromJson(
   store: ConfigStore,
   json: string,
): string[] {
   const result = parseConfigJson(json);
   setConfig(store, result.config);
   return result.warnings;
}

/**
 * Load a config from a FileEntry that was already added to the file store
 * (e.g. via user file upload detected as AcbConfigJson).
 *
 * Adopts the given file entry as the config's file drawer entry, removing
 * the previous config file entry if one existed. Updates the entry's data
 * with the re-serialized (canonical) JSON.
 *
 * Returns warnings from validation.  Throws on parse errors.
 */
export function loadConfigFromFileEntry(
   store: ConfigStore,
   entry: FileEntry,
): string[] {
   const json = new TextDecoder().decode(entry.data);
   const result = parseConfigJson(json);

   // Remove old config file entry if it's a different entry.
   if (store.fileEntryId !== null && store.fileEntryId !== entry.id) {
      removeConfigFileEntry(store);
   }

   store.config = result.config;
   store.fileEntryId = entry.id;
   saveConfigToStorage(result.config);

   // Update the entry's data with canonical JSON and mark as downloadable.
   const canonical = serializeConfig(result.config);
   entry.data = new TextEncoder().encode(canonical);
   entry.isDownloadable = true;

   return result.warnings;
}

/**
 * Clear the current config from the store, localStorage, and file drawer.
 */
export function clearConfig(store: ConfigStore): void {
   store.config = null;
   localStorage.removeItem(STORAGE_KEY);
   configExists.value = false;
   removeConfigFileEntry(store);
}

// -- File drawer integration --

function syncConfigToFileDrawer(store: ConfigStore): void {
   if (!store.config) return;

   const json = serializeConfig(store.config);
   const data = new TextEncoder().encode(json);
   const fileStore = getFileManagerStore();

   // Update existing entry if present.
   if (store.fileEntryId !== null) {
      const existing = fileStore.files.find(
         f => f.id === store.fileEntryId
      );
      if (existing) {
         existing.data = data;
         return;
      }
      // Entry was removed externally; fall through to re-add.
      store.fileEntryId = null;
   }

   const entry: FileEntry = fileStore.addFile({
      name: DEFAULT_CONFIG_FILE_NAME,
      kind: FileKind.AcbConfigJson,
      isDownloadable: true,
      useChecked: false,
      data,
   });
   store.fileEntryId = entry.id;
   modifyDrawerNotificationForUserAddedFiles(fileStore);
}

function removeConfigFileEntry(store: ConfigStore): void {
   if (store.fileEntryId === null) return;
   const fileStore = getFileManagerStore();
   fileStore.removeFiles([store.fileEntryId]);
   store.fileEntryId = null;
}
