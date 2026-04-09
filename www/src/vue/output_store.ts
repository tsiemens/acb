import { reactive } from 'vue';
import { AppFunctionMode } from '../common/acb_app_types.js';
import type { RenderTable } from '../acb_wasm_types.js';

export enum InactiveFilterMode {
   DimRows = "dim_rows",
   HideSecurities = "hide_securities",
   HideRows = "hide_rows",
}

export enum AcbOutputViewMode {
   SecurityTables = "security_tables",
   Aggregate = "aggregate",
   Summary = "summary",
   Text = "text",
}

export interface OutputStore {
   activeViewMode: AcbOutputViewMode;
   selectableViewModes: AcbOutputViewMode[];
   isLoading: boolean;
   textOutput: string;
   summaryTable: RenderTable | null;
   aggregateTable: RenderTable | null;
   securityTables: Map<string, RenderTable> | null;
   highlightedYear: string | null;
   inactiveFilterMode: InactiveFilterMode;
}

let store: OutputStore | null = null;

export function getOutputStore(): OutputStore {
   if (!store) {
      store = reactive({
         activeViewMode: AcbOutputViewMode.SecurityTables,
         selectableViewModes: selectableViewModesForAppFunction(AppFunctionMode.Calculate),
         isLoading: false,
         textOutput: '',
         summaryTable: null,
         aggregateTable: null,
         securityTables: null,
         highlightedYear: null,
         inactiveFilterMode: InactiveFilterMode.DimRows,
      });
   }
   return store;
}

export function selectableViewModesForAppFunction(funcMode: AppFunctionMode): AcbOutputViewMode[] {
   switch (funcMode) {
      case AppFunctionMode.Calculate:
         return [
            AcbOutputViewMode.SecurityTables,
            AcbOutputViewMode.Aggregate,
            AcbOutputViewMode.Text,
         ];
      case AppFunctionMode.TxSummary:
      case AppFunctionMode.TallyShares:
         return [
            AcbOutputViewMode.Summary,
            AcbOutputViewMode.Text,
         ];
   }
}

export function setAppFunctionViewMode(funcMode: AppFunctionMode): void {
   const store = getOutputStore();
   const modes = selectableViewModesForAppFunction(funcMode);
   store.selectableViewModes = modes;
   if (!modes.includes(store.activeViewMode)) {
      store.activeViewMode = modes[0];
   }
}

const VIEW_MODE_LABELS: Record<AcbOutputViewMode, string> = {
   [AcbOutputViewMode.SecurityTables]: "Securities",
   [AcbOutputViewMode.Summary]: "Summary",
   [AcbOutputViewMode.Aggregate]: "Aggregate",
   [AcbOutputViewMode.Text]: "Raw Text",
};

export function getViewModeLabel(mode: AcbOutputViewMode): string {
   return VIEW_MODE_LABELS[mode];
}
