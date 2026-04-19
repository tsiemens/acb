import { reactive } from 'vue';
import { getCurrentTaxYear } from '../tax_logic.js';
import type { AcbTaxEntry } from '../tampermonkey_gen.js';

export interface TampermonkeyDialogOptions {
   years: number[];
   affiliates: string[];
   securities: string[];
   entries: AcbTaxEntry[];
}

export interface TampermonkeyDialogResult {
   year: number;
   affiliate: string | null;
   securities: string[];
}

export interface TampermonkeyDialogStore {
   active: boolean;
   selectedYear: number;
   selectedAffiliate: string | null;
   selectedSecurities: Record<string, boolean>;
   yearOptions: number[];
   affiliateOptions: string[];
   securityOptions: string[];
   entries: AcbTaxEntry[];
   _resolve: ((result: TampermonkeyDialogResult | null) => void) | null;
}

let _store: TampermonkeyDialogStore | null = null;

export function getTampermonkeyDialogStore(): TampermonkeyDialogStore {
   if (!_store) {
      _store = reactive({
         active: false,
         selectedYear: getCurrentTaxYear(),
         selectedAffiliate: null,
         selectedSecurities: {},
         yearOptions: [],
         affiliateOptions: [],
         securityOptions: [],
         entries: [],
         _resolve: null,
      });
   }
   return _store;
}

/**
 * Opens the Tampermonkey export dialog and returns a promise that resolves
 * to the selected year, affiliate, and included securities, or null if
 * dismissed.
 */
export function showTampermonkeyExportDialog(
   options: TampermonkeyDialogOptions,
): Promise<TampermonkeyDialogResult | null> {
   const store = getTampermonkeyDialogStore();
   if (store._resolve) {
      store._resolve(null);
   }
   store.yearOptions = options.years;
   store.affiliateOptions = options.affiliates;
   store.securityOptions = options.securities;
   store.entries = options.entries;
   const taxYear = getCurrentTaxYear();
   store.selectedYear = options.years.includes(taxYear) ? taxYear :
      (options.years.length > 0 ? options.years[0] : taxYear);
   store.selectedAffiliate = options.affiliates.length > 1 ? options.affiliates[0] : null;
   const selected: Record<string, boolean> = {};
   for (const sec of options.securities) {
      selected[sec] = true;
   }
   store.selectedSecurities = selected;
   store.active = true;
   return new Promise<TampermonkeyDialogResult | null>((resolve) => {
      store._resolve = resolve;
   });
}

export function resolveTampermonkeyDialog(result: TampermonkeyDialogResult | null): void {
   const store = getTampermonkeyDialogStore();
   const resolve = store._resolve;
   store.active = false;
   store._resolve = null;
   if (resolve) {
      resolve(result);
   }
}
