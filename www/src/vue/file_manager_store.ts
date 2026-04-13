import { reactive, watchEffect } from 'vue';
import { getTabStore, tabs, TabId, type TabIdType } from './tab_store.js';

export enum FileKind {
   AcbTxCsv    = 'AcbTxCsv',
   QuestradeXlsx = 'QuestradeXls',
   EtradeTradeConfirmationPdf = 'EtradeTradeConfirmationPdf',
   EtradeBenefitPdf = 'EtradeBenefitPdf',
   EtradeBenefitsExcel = 'EtradeBenefitsExcel',
   RbcDiCsv    = 'RbcDiCsv',
   OutputText  = 'OutputText',
   AcbOutputZip = 'AcbOutputZip',
   AcbConfigJson = 'AcbConfigJson',
   TampermonkeyScript = 'TampermonkeyScript',
   GenericPdf  = 'GenericPdf',
   Other       = 'Other',
}

interface FileKindMeta {
   label: string;               // Short display name shown in tags and filter pills
   isInput: boolean;            // Whether the file can be "used" as app input
   isDownloadableDefault: boolean; // Default downloadability for this kind
}

const FILE_KIND_META: Record<FileKind, FileKindMeta> = {
   [FileKind.AcbTxCsv]:                   { label: 'ACB TX csv',           isInput: true,  isDownloadableDefault: false },
   [FileKind.QuestradeXlsx]:              { label: 'Questrade xlsx',       isInput: true,  isDownloadableDefault: false },
   [FileKind.EtradeTradeConfirmationPdf]: { label: 'E*TRADE Trade pdf',   isInput: true,  isDownloadableDefault: false },
   [FileKind.EtradeBenefitPdf]:           { label: 'E*TRADE Benefit pdf',  isInput: true,  isDownloadableDefault: false },
   [FileKind.EtradeBenefitsExcel]:        { label: 'E*TRADE Benefits xlsx', isInput: true,  isDownloadableDefault: false },
   [FileKind.RbcDiCsv]:                   { label: 'RBC DI csv',           isInput: true,  isDownloadableDefault: false },
   [FileKind.OutputText]:                 { label: 'Text',                 isInput: false, isDownloadableDefault: true  },
   [FileKind.AcbOutputZip]:               { label: 'ACB Output',           isInput: false, isDownloadableDefault: true  },
   [FileKind.AcbConfigJson]:              { label: 'ACB Config',            isInput: false, isDownloadableDefault: true  },
   [FileKind.TampermonkeyScript]:         { label: 'Tampermonkey JS',       isInput: false, isDownloadableDefault: true  },
   [FileKind.GenericPdf]:                 { label: 'PDF',                  isInput: false, isDownloadableDefault: false },
   [FileKind.Other]:                      { label: 'Other',                isInput: false, isDownloadableDefault: false },
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
   /** Pre-extracted PDF page texts (set during file load for PDF files). */
   pdfPageTexts?: string[];
   /** True while async PDF detection is in progress. */
   isDetecting?: boolean;
}

export interface FileManagerState {
   files: FileEntry[];
   hasNotification: boolean;
   isExpanded: boolean;
   // Which input kinds are relevant to the active tab. The Use checkbox is
   // highlighted for relevant kinds and neutral for others. Derived from the
   // tab store via watchEffect.
   relevantInputKinds: Set<FileKind>;
   addFile(
      entry: Omit<FileEntry, 'id' | 'isSelected'>,
      options?: { skipDedup?: boolean },
   ): FileEntry;
   removeFiles(ids: number[]): void;
   clearSelection(): void;
   setSelectedByIds(ids: Set<number>): void;
}

export function relevantInputKindsForTab(tabId: TabIdType): Set<FileKind> {
   switch (tabId) {
      case TabId.AcbCalc:
         return new Set([FileKind.AcbTxCsv]);
      case TabId.BrokerConvert:
         return new Set([
            FileKind.QuestradeXlsx,
            FileKind.EtradeTradeConfirmationPdf,
            FileKind.EtradeBenefitPdf,
            FileKind.EtradeBenefitsExcel,
            FileKind.RbcDiCsv,
         ]);
      case TabId.Configuration:
         return new Set();
   }
}

function makeState(): FileManagerState {
   return reactive({
      files: [] as FileEntry[],
      hasNotification: false,
      isExpanded: false,
      relevantInputKinds: relevantInputKindsForTab(getTabStore().activeTab),
      addFile(
         entry: Omit<FileEntry, 'id' | 'isSelected'>,
         options?: { skipDedup?: boolean },
      ): FileEntry {
         return addFile(this, entry, options);
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

      // Update per-tab run-enabled state based purely on file state.
      watchEffect(() => {
         const anyDetecting = s.files.some(f => f.isDetecting);
         for (const tab of tabs) {
            const kinds = relevantInputKindsForTab(tab.id);
            const enabled = !anyDetecting && s.files.some(f =>
               FileKind.isInput(f.kind) &&
               f.useChecked &&
               !f.warning &&
               kinds.has(f.kind)
            );
            tabStore.runEnabledByTab.set(tab.id, enabled);
         }
      });

      // Glow background tabs when their run button transitions to enabled.
      const prevEnabled: Partial<Record<TabIdType, boolean>> = {};
      watchEffect(() => {
         for (const tab of tabs) {
            const enabled = tabStore.runEnabledByTab.get(tab.id) ?? false;
            if (enabled && !prevEnabled[tab.id] && tab.id !== tabStore.activeTab) {
               tabStore.glowingTabs.add(tab.id);
            }
            prevEnabled[tab.id] = enabled;
         }
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

/** If `name` already exists among `existingNames`, return a deduplicated
 *  version by appending " (N)" before the extension (or at the end if there
 *  is no extension).  E.g. "foo.csv" → "foo (2).csv", "bar" → "bar (2)".
 *  Increments N until the result is unique. */
export function deduplicateFileName(name: string, existingNames: Set<string>): string {
   if (!existingNames.has(name)) {
      return name;
   }

   const dotIdx = name.lastIndexOf('.');
   const stem = dotIdx > 0 ? name.slice(0, dotIdx) : name;
   const ext  = dotIdx > 0 ? name.slice(dotIdx) : '';

   for (let n = 2; ; n++) {
      const candidate = `${stem} (${String(n)})${ext}`;
      if (!existingNames.has(candidate)) {
         return candidate;
      }
   }
}

export function addFile(
   state: FileManagerState,
   entry: Omit<FileEntry, 'id' | 'isSelected'>,
   options?: { skipDedup?: boolean },
): FileEntry {
   let name = entry.name;
   if (!options?.skipDedup) {
      const existingNames = new Set(state.files.map((f) => f.name));
      name = deduplicateFileName(entry.name, existingNames);
   }
   const file: FileEntry = { ...entry, name, id: nextId++, isSelected: false };
   state.files.push(file);
   // Return the reactive proxy (last element), not the raw object.
   // Mutations on the raw object bypass Vue's reactivity tracking.
   // (Typescript doesn't distinguish between Proxy<T> and T, but the
   // runtime objects are different.)
   return state.files[state.files.length - 1];
}
