import { reactive } from 'vue';
import { AppFunctionMode } from '../common/acb_app_types.js';

function getDefaultDate(mode: AppFunctionMode): Date {
   const now = new Date();
   if (mode === AppFunctionMode.TxSummary) {
      const lastYear = now.getFullYear() - 1;
      return new Date(`${lastYear.toString()}-12-31`);
   }
   return now;
}

function dateToInputString(date: Date): string {
   return date.toISOString().split('T')[0];
}

function modeNeedsDate(mode: AppFunctionMode): boolean {
   return mode === AppFunctionMode.TxSummary ||
          mode === AppFunctionMode.TallyShares;
}

export interface AppInputStore {
   functionMode: AppFunctionMode;
   /** ISO date string (yyyy-mm-dd) for the date picker */
   summaryDateStr: string;
   printFullValues: boolean;

   /** Per-mode saved dates, so switching modes restores the last picked date */
   lastPickedDates: Map<AppFunctionMode, string>;

   // Broker convert options
   /** Extract raw PDF data without harmonizing benefits/trades (E*TRADE only) */
   extractOnly: boolean;
   /** Filter out .FX (foreign exchange) transactions */
   noFx: boolean;
}

let _store: AppInputStore | null = null;

export function getAppInputStore(): AppInputStore {
   if (!_store) {
      const defaultMode = AppFunctionMode.Calculate;
      _store = reactive({
         functionMode: defaultMode,
         summaryDateStr: dateToInputString(getDefaultDate(defaultMode)),
         printFullValues: false,
         lastPickedDates: new Map(),
         extractOnly: false,
         noFx: false,
      }) as AppInputStore;
   }
   return _store;
}

/** Called when the mode changes to update the date picker value. */
export function updateDateForMode(store: AppInputStore): void {
   const mode = store.functionMode;
   const saved = store.lastPickedDates.get(mode);
   if (saved) {
      store.summaryDateStr = saved;
   } else {
      store.summaryDateStr = dateToInputString(getDefaultDate(mode));
   }
}

/** Whether the current mode needs a date picker. */
export function shouldShowDatePicker(store: AppInputStore): boolean {
   return modeNeedsDate(store.functionMode);
}

/** Get the summary date as a Date object, or a default if empty. */
export function getSummaryDate(store: AppInputStore): Date {
   if (store.summaryDateStr) {
      return new Date(store.summaryDateStr);
   }
   return getDefaultDate(store.functionMode);
}
