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
   selectedAffiliate: string | null;
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
         selectedAffiliate: null,
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

/**
 * Strip the registered suffix "(R)" and any cost pool marker `[...]` to get the
 * base affiliate name. Cost pools (e.g. "Default [RSU 2026-02-20]") thereby
 * cluster under their parent affiliate in filters rather than each appearing as
 * a separate entry. Mirrors `Affiliate::base_name_normalized` in Rust.
 */
export function affiliateBaseName(affiliate: string): string {
   return affiliate
      .replace(/\s*\[[^\]]*\]/, '')
      .replace(/\s*\(R\)\s*$/i, '')
      .trim();
}

/**
 * Extract the cost pool tag from an affiliate's string representation: the
 * contents of the bracketed marker `[...]` (e.g. "7(1.31) - RSU 123456
 * 2017-02-01" for "Default [7(1.31) - RSU 123456 2017-02-01]"). Returns null
 * when there is no cost pool marker or it is empty. Complements
 * affiliateBaseName, which strips this same marker.
 */
export function affiliateCostPoolTag(affiliate: string): string | null {
   const m = /\[([^\]]*)\]/.exec(affiliate);
   if (!m) return null;
   const tag = m[1].trim();
   return tag.length > 0 ? tag : null;
}

/**
 * Check if a row's affiliate value matches the selected affiliate filter.
 * Matches on base name (ignoring "(R)" suffix).
 */
export function affiliateMatches(rowAffiliate: string, selected: string): boolean {
   return affiliateBaseName(rowAffiliate) === selected;
}
