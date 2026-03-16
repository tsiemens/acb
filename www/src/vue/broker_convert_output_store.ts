import { reactive } from 'vue';
import type { RenderTable } from '../acb_wasm_types.js';

export enum BrokerConvertViewMode {
   Transactions = 'transactions',
   RawText = 'raw_text',
}

const VIEW_MODE_LABELS: Record<BrokerConvertViewMode, string> = {
   [BrokerConvertViewMode.Transactions]: 'Transactions',
   [BrokerConvertViewMode.RawText]: 'Raw Text',
};

export function getBrokerConvertViewModeLabel(mode: BrokerConvertViewMode): string {
   return VIEW_MODE_LABELS[mode];
}

export const BROKER_CONVERT_VIEW_MODES: ReadonlyArray<BrokerConvertViewMode> = [
   BrokerConvertViewMode.Transactions,
   BrokerConvertViewMode.RawText,
];

export interface BrokerConvertOutputStore {
   activeViewMode: BrokerConvertViewMode;
   isLoading: boolean;
   textOutput: string;
   transactionsTable: RenderTable | null;
}

let _store: BrokerConvertOutputStore | null = null;

export function getBrokerConvertOutputStore(): BrokerConvertOutputStore {
   if (!_store) {
      _store = reactive({
         activeViewMode: BrokerConvertViewMode.Transactions,
         isLoading: false,
         textOutput: '',
         transactionsTable: null,
      });
   }
   return _store;
}
