import { reactive, ref, watchEffect } from 'vue';
import { make_default_config, parse_config, serialize_config } from '../pkg/acb_wasm.js';
import { FileKind, getFileManagerStore, modifyDrawerNotificationForUserAddedFiles, type FileEntry } from './file_manager_store.js';

// -- Config types (mirrors Rust AcbConfig / AccountBindings) --

export interface AccountBindings {
   questrade: Map<string, string>;
   rbc_di: Map<string, string>;
   etrade: Map<string, string>;
}

export interface AcbConfig {
   version: number;
   account_bindings: AccountBindings;
   symbol_renames: Map<string, string>;
}

// Wire format types — plain Records as JSON serializes/deserializes them.
interface AccountBindingsWire {
   questrade: Record<string, string>;
   rbc_di: Record<string, string>;
   etrade: Record<string, string>;
}

interface AcbConfigWire {
   version: number;
   account_bindings: AccountBindingsWire;
   symbol_renames: Record<string, string>;
}

function recordToMap(r: Record<string, string>): Map<string, string> {
   return new Map(Object.entries(r));
}

function mapToRecord(m: Map<string, string>): Record<string, string> {
   return Object.fromEntries(m);
}

function configFromWire(wire: AcbConfigWire): AcbConfig {
   return {
      version: wire.version,
      account_bindings: {
         questrade: recordToMap(wire.account_bindings.questrade),
         rbc_di: recordToMap(wire.account_bindings.rbc_di),
         etrade: recordToMap(wire.account_bindings.etrade),
      },
      symbol_renames: recordToMap(wire.symbol_renames),
   };
}

function configToWire(config: AcbConfig): AcbConfigWire {
   return {
      version: config.version,
      account_bindings: {
         questrade: mapToRecord(config.account_bindings.questrade),
         rbc_di: mapToRecord(config.account_bindings.rbc_di),
         etrade: mapToRecord(config.account_bindings.etrade),
      },
      symbol_renames: mapToRecord(config.symbol_renames),
   };
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
      config: configFromWire(result.config as AcbConfigWire),
      // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
      warnings: (result.warnings ?? []) as string[],
   };
}

/** Serialize an AcbConfig to pretty-printed JSON via WASM. */
export function serializeConfig(config: AcbConfig): string {
   return serialize_config(configToWire(config));
}

/** Returns a default (empty) AcbConfig, sourced from the WASM layer. */
export function makeDefaultConfig(): AcbConfig {
   return configFromWire(make_default_config() as AcbConfigWire);
}

/** Add or update a symbol rename entry. */
export function setSymbolRename(store: ConfigStore, from: string, to: string): void {
   const config = store.config ?? makeDefaultConfig();
   config.symbol_renames.set(from, to);
   setConfig(store, config);
}

/** Remove a symbol rename entry (no-op if not present). */
export function removeSymbolRename(store: ConfigStore, from: string): void {
   if (!store.config) return;
   store.config.symbol_renames.delete(from);
   setConfig(store, store.config);
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

      // Watch for the config file entry being removed from the drawer
      // (e.g. user deletes it) and clear the cached config accordingly.
      const s = _store;
      watchEffect(() => {
         if (s.fileEntryId === null || s.config === null) return;
         const fileStore = getFileManagerStore();
         const exists = fileStore.files.some(f => f.id === s.fileEntryId);
         if (!exists) {
            s.config = null;
            s.fileEntryId = null;
            localStorage.removeItem(STORAGE_KEY);
            configExists.value = false;
         }
      });
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
   // Ensure a new reference so Vue's reactivity triggers even when the
   // caller mutated the existing config object in-place.
   store.config = (config === store.config) ? { ...config } : config;
   saveConfigToStorage(store.config);
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
