import { reactive, watch } from 'vue';

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
   /** Whether the run button is enabled for each tab. Set by file_manager_store. */
   runEnabledByTab: Map<TabIdType, boolean>;
   /** Tabs whose state changed while they were in the background. */
   glowingTabs: Set<TabIdType>;
}

let _store: TabStore | null = null;

export function getTabStore(): TabStore {
   if (!_store) {
      _store = reactive({
         activeTab: TabId.AcbCalc,
         runEnabledByTab: new Map<TabIdType, boolean>(
            tabs.map(t => [t.id, false]),
         ),
         glowingTabs: new Set<TabIdType>(),
      }) as TabStore;

      const s = _store;
      watch(() => s.activeTab, (newTab) => {
         s.glowingTabs.delete(newTab);
      });
   }
   return _store;
}
