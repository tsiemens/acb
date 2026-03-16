import { reactive } from 'vue';

export const TabId = {
   AcbCalc: 'acb-calc',
   BrokerConvert: 'broker-convert',
} as const;

export type TabIdType = typeof TabId[keyof typeof TabId];

export const tabs: ReadonlyArray<{ id: TabIdType; label: string }> = [
   { id: TabId.AcbCalc, label: 'ACB Calculator' },
   { id: TabId.BrokerConvert, label: 'Broker Activity Convert' },
];

export interface TabStore {
   activeTab: TabIdType;
}

let _store: TabStore | null = null;

export function getTabStore(): TabStore {
   if (!_store) {
      _store = reactive({
         activeTab: TabId.AcbCalc,
      }) as TabStore;
   }
   return _store;
}
