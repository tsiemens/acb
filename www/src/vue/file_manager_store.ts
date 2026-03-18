import { reactive, watchEffect } from 'vue';
import { getTabStore, TabId, type TabIdType } from './tab_store.js';

export enum FileKind {
   AcbTxCsv    = 'AcbTxCsv',
   QuestradeXlsx = 'QuestradeXls',
   OutputText  = 'OutputText',
   AcbOutputZip = 'AcbOutputZip',
   Other       = 'Other',
}

interface FileKindMeta {
   label: string;               // Short display name shown in tags and filter pills
   isInput: boolean;            // Whether the file can be "used" as app input
   isDownloadableDefault: boolean; // Default downloadability for this kind
}

const FILE_KIND_META: Record<FileKind, FileKindMeta> = {
   [FileKind.AcbTxCsv]:      { label: 'ACB TX csv',     isInput: true,  isDownloadableDefault: false },
   [FileKind.QuestradeXlsx]: { label: 'Questrade xlsx', isInput: true,  isDownloadableDefault: false },
   [FileKind.OutputText]:    { label: 'Text',           isInput: false, isDownloadableDefault: true  },
   [FileKind.AcbOutputZip]:  { label: 'ACB Output',     isInput: false, isDownloadableDefault: true  },
   [FileKind.Other]:         { label: 'Other',          isInput: false, isDownloadableDefault: false },
};

// Declaration/namespace merge, so we can add static methods to the FileKind enum.
// eslint-disable-next-line @typescript-eslint/no-namespace
export namespace FileKind {
   export function label(kind: FileKind): string {
      return FILE_KIND_META[kind].label;
   }
   export function isInput(kind: FileKind): boolean {
      return FILE_KIND_META[kind].isInput;
   }
   export function isDownloadableDefault(kind: FileKind): boolean {
      return FILE_KIND_META[kind].isDownloadableDefault;
   }
}

export interface FileEntry {
   id: number;           // An arbitrary ID, set by the app.
   name: string;
   kind: FileKind;
   isDownloadable: boolean; // Can be set independently of kind (e.g. generated inputs)
   useChecked: boolean;  // Only meaningful when fileKindIsInput(kind) is true
   isSelected: boolean;
   warning?: string;     // Set when an error is detected reading/processing the file
   data: Uint8Array;
}

export interface FileManagerState {
   files: FileEntry[];
   hasNotification: boolean;
   isExpanded: boolean;
   // Which input kinds are relevant to the active tab. The Use checkbox is
   // highlighted for relevant kinds and neutral for others. Derived from the
   // tab store via watchEffect.
   relevantInputKinds: Set<FileKind>;
   addFile(entry: Omit<FileEntry, 'id' | 'isSelected'>): FileEntry;
   removeFiles(ids: number[]): void;
   clearSelection(): void;
   setSelectedByIds(ids: Set<number>): void;
}

function relevantInputKindsForTab(tabId: TabIdType): Set<FileKind> {
   switch (tabId) {
      case TabId.AcbCalc:
         return new Set([FileKind.AcbTxCsv]);
      case TabId.BrokerConvert:
         return new Set([FileKind.QuestradeXlsx]);
   }
}

function makeState(): FileManagerState {
   return reactive({
      files: [] as FileEntry[],
      hasNotification: false,
      isExpanded: false,
      relevantInputKinds: relevantInputKindsForTab(getTabStore().activeTab),
      addFile(entry: Omit<FileEntry, 'id' | 'isSelected'>): FileEntry {
         return addFile(this, entry);
      },
      removeFiles(ids: number[]): void {
         const idSet = new Set(ids);
         this.files = this.files.filter((f) => !idSet.has(f.id));
      },
      clearSelection(): void {
         this.files.forEach((f) => (f.isSelected = false));
      },
      setSelectedByIds(ids: Set<number>): void {
         this.files.forEach((f) => (f.isSelected = ids.has(f.id)));
      },
   });
}

let store: FileManagerState | null = null;
let nextId = 1;

export function getFileManagerStore(): FileManagerState {
   if (!store) {
      store = makeState();
      const s = store;
      const tabStore = getTabStore();
      watchEffect(() => {
         s.relevantInputKinds = relevantInputKindsForTab(tabStore.activeTab);
      });
   }
   return store;
}

// Call after adding files on behalf of the user. Sets the notification dot
// only when the drawer is closed — no need to notify if they can already see
// the new entries.
export function modifyDrawerNotificationForUserAddedFiles(store: FileManagerState): void {
   if (!store.isExpanded) {
      store.hasNotification = true;
   }
}

export function addFile(
   state: FileManagerState,
   entry: Omit<FileEntry, 'id' | 'isSelected'>,
): FileEntry {
   const file: FileEntry = { ...entry, id: nextId++, isSelected: false };
   state.files.push(file);
   return file;
}
