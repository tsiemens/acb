<template>
  <div class="config-panel">
    <!-- Existing bindings -->
    <div v-if="allBindings.length > 0" class="bindings-section">
      <table class="bindings-table">
        <thead>
          <tr>
            <th>Broker</th>
            <th>Account #</th>
            <th>Affiliate</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="b in allBindings" :key="b.broker + ':' + b.accountNum">
            <td class="broker-cell">{{ brokerLabel(b.broker) }}</td>
            <td class="account-cell">{{ b.accountNum }}</td>
            <td class="affiliate-cell">
              <input
                type="text"
                class="affiliate-input"
                :value="b.affiliateName"
                @change="onEditBinding(b.broker, b.accountNum, ($event.target as HTMLInputElement).value)"
                placeholder="Default"
              >
            </td>
            <td class="action-cell">
              <button
                class="remove-btn"
                title="Remove binding"
                @click="onRemoveBinding(b.broker, b.accountNum)"
              >&times;</button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Unbound accounts detected from uploaded files -->
    <div v-if="unboundAccounts.length > 0" class="unbound-section">
      <div class="unbound-label">Unbound accounts from files (implicitly bound to 'Default'):</div>
      <div v-for="ua in unboundAccounts" :key="ua.broker + ':' + ua.accountNum" class="unbound-row">
        <span class="unbound-info">{{ brokerLabel(ua.broker) }} {{ ua.accountNum }}
          <span v-if="ua.accountType" class="account-type">({{ ua.accountType }})</span>
        </span>
        <input
          type="text"
          class="affiliate-input"
          placeholder="Affiliate name or 'Default'"
          @keydown.enter="onBindUnbound(ua.broker, ua.accountNum, ($event.target as HTMLInputElement).value, $event)"
        >
        <button
          class="bind-btn"
          @click="onBindUnboundFromSibling(ua.broker, ua.accountNum, $event)"
        >+</button>
      </div>
    </div>

    <!-- Manual add -->
    <div class="manual-add">
      <div class="manual-add-label">Add binding:</div>
      <div class="manual-add-row">
        <select v-model="addBroker" class="add-select">
          <option value="questrade">Questrade</option>
          <option value="rbc_di">RBC DI</option>
          <option value="etrade">E*TRADE</option>
        </select>
        <input
          type="text"
          v-model="addAccountNum"
          placeholder="Account #"
          class="add-account-input"
        >
        <input
          type="text"
          v-model="addAffiliateName"
          placeholder="Affiliate"
          class="add-affiliate-input"
        >
        <button
          class="bind-btn"
          :disabled="!addAccountNum.trim() || !addAffiliateName.trim()"
          @click="onManualAdd"
        >+</button>
      </div>
    </div>

    <div v-if="allBindings.length === 0 && unboundAccounts.length === 0" class="empty-hint">
      No account bindings configured. Add broker files to detect accounts,
      or add bindings manually. These will be saved across sessions, and can be
      restored by re-uploading an acb-config.json file.
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref, computed, watch } from 'vue';
import {
   getConfigStore, setConfig, makeDefaultConfig,
   type AcbConfig, type AccountBindings, type ConfigStore,
} from './config_store.js';
import { getFileManagerStore, FileKind } from './file_manager_store.js';
import { extract_account_numbers } from '../pkg/acb_wasm.js';

interface Binding {
   broker: string;
   accountNum: string;
   affiliateName: string;
}

interface UnboundAccount {
   broker: string;
   accountNum: string;
   accountType: string;
}

interface AccountExtractionResult {
   accounts: Array<{ broker: string; account_num: string; account_type: string }>;
   warnings: string[];
}

const BROKER_LABELS: Record<keyof AccountBindings, string> = {
   questrade: 'Questrade',
   rbc_di: 'RBC DI',
   etrade: 'E*TRADE',
};

function brokerLabel(key: string): string {
   return (BROKER_LABELS as Record<string, string>)[key] ?? key;
}

function ensureConfig(store: ConfigStore): AcbConfig {
   if (store.config) return store.config;
   return makeDefaultConfig();
}

function sanitizeAffiliateName(raw: string): string {
   return raw.trim().replace(/\s*\([rR]\)\s*$/, '');
}

function getBindingsMap(config: AcbConfig, broker: string): Record<string, string> {
   return config.account_bindings[broker as keyof AccountBindings];
}

export default defineComponent({
   name: 'AccountBindingsEditor',
   setup() {
      const configStore = getConfigStore();
      const fileStore = getFileManagerStore();

      const addBroker = ref('questrade');
      const addAccountNum = ref('');
      const addAffiliateName = ref('');

      // Extracted accounts from uploaded files (updated reactively).
      const extractedAccounts = ref<UnboundAccount[]>([]);

      // All current bindings from the config.
      const allBindings = computed<Binding[]>(() => {
         const config = configStore.config;
         if (!config) return [];
         const result: Binding[] = [];
         for (const broker of Object.keys(BROKER_LABELS)) {
            const map = getBindingsMap(config, broker);
            for (const [acctNum, affName] of Object.entries(map)) {
               result.push({
                  broker,
                  accountNum: acctNum,
                  affiliateName: affName,
               });
            }
         }
         return result;
      });

      // Unbound = extracted accounts not in current config.
      const unboundAccounts = computed<UnboundAccount[]>(() => {
         const config = configStore.config;
         return extractedAccounts.value.filter(ea => {
            if (!config) return true;
            const map = getBindingsMap(config, ea.broker);
            return !(ea.accountNum in map);
         });
      });

      // Watch for file changes to re-extract accounts.
      function extractAccounts() {
         const brokerFiles = fileStore.files.filter(
            f => (f.kind === FileKind.QuestradeXlsx ||
                  f.kind === FileKind.RbcDiCsv ||
                  f.kind === FileKind.EtradeBenefitsExcel) &&
                 !f.warning
         );
         const pdfFiles = fileStore.files.filter(
            f => (f.kind === FileKind.EtradeTradeConfirmationPdf ||
                  f.kind === FileKind.EtradeBenefitPdf) &&
                 !f.warning && f.pdfPageTexts
         );

         if (brokerFiles.length === 0 && pdfFiles.length === 0) {
            extractedAccounts.value = [];
            return;
         }

         try {
            const fileDatas = brokerFiles.map(f => new Uint8Array(f.data));
            const fileNames = brokerFiles.map(f => f.name);
            // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
            const pdfTexts = pdfFiles.map(f => f.pdfPageTexts!.join('\n'));
            const pdfNames = pdfFiles.map(f => f.name);

            // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
            const jsResult = extract_account_numbers(fileDatas, fileNames, pdfTexts, pdfNames);
            const result = jsResult as AccountExtractionResult;

            if (result.warnings.length > 0) {
               for (const w of result.warnings) {
                  console.warn('Account extraction warning:', w);
               }
            }

            extractedAccounts.value = result.accounts.map(a => ({
               broker: a.broker,
               accountNum: a.account_num,
               accountType: a.account_type,
            }));
         } catch (err) {
            console.warn('Account extraction failed:', err);
            extractedAccounts.value = [];
         }
      }

      // Watch file list (length + detecting status) to re-extract.
      watch(
         () => {
            const files = fileStore.files;
            return files.map(f => `${f.id}:${f.kind}:${f.isDetecting ?? false}`).join(',');
         },
         () => extractAccounts(),
         { immediate: true },
      );

      function updateConfig(mutate: (config: AcbConfig) => void) {
         const config = ensureConfig(configStore);
         mutate(config);
         setConfig(configStore, config);
      }

      function onEditBinding(broker: string, accountNum: string, newName: string) {
         const afName = sanitizeAffiliateName(newName);
         if (!afName) {
            onRemoveBinding(broker, accountNum);
            return;
         }
         updateConfig(config => {
            getBindingsMap(config, broker)[accountNum] = afName;
         });
      }

      function onRemoveBinding(broker: string, accountNum: string) {
         updateConfig(config => {
            const map = getBindingsMap(config, broker);
            delete map[accountNum];
         });
      }

      function onBindUnbound(broker: string, accountNum: string, value: string, event: Event) {
         const afName = sanitizeAffiliateName(value);
         if (!afName) return;
         updateConfig(config => {
            getBindingsMap(config, broker)[accountNum] = afName;
         });
         (event.target as HTMLInputElement).value = '';
      }

      function onBindUnboundFromSibling(broker: string, accountNum: string, event: Event) {
         const btn = event.target as HTMLElement;
         const row = btn.closest('.unbound-row');
         if (!row) return;
         const input = row.querySelector('.affiliate-input') as HTMLInputElement | null;
         if (!input) return;
         const afName = sanitizeAffiliateName(input.value);
         if (!afName) return;
         updateConfig(config => {
            getBindingsMap(config, broker)[accountNum] = afName;
         });
         input.value = '';
      }

      function onManualAdd() {
         const acct = addAccountNum.value.trim();
         const afName = sanitizeAffiliateName(addAffiliateName.value);
         if (!acct || !afName) return;
         updateConfig(config => {
            getBindingsMap(config, addBroker.value)[acct] = afName;
         });
         addAccountNum.value = '';
         addAffiliateName.value = '';
      }

      return {
         allBindings,
         unboundAccounts,
         addBroker,
         addAccountNum,
         addAffiliateName,
         brokerLabel,
         onEditBinding,
         onRemoveBinding,
         onBindUnbound,
         onBindUnboundFromSibling,
         onManualAdd,
      };
   },
});
</script>

<style scoped>
.config-panel {
  font-size: 13px;
}

.bindings-section {
  margin-bottom: 12px;
}

.bindings-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}

.bindings-table th {
  text-align: left;
  font-weight: 600;
  padding: 4px 6px;
  border-bottom: 1px solid #d1d5db;
  color: #374151;
}

.bindings-table td {
  padding: 4px 6px;
  border-bottom: 1px solid #e5e7eb;
}

.broker-cell {
  white-space: nowrap;
  color: #6b7280;
}

.account-cell {
  font-family: monospace;
  font-size: 12px;
}

.affiliate-input {
  width: 100%;
  padding: 3px 6px;
  border: 1px solid #d1d5db;
  border-radius: 4px;
  font-size: 12px;
}

.affiliate-input:focus {
  outline: none;
  border-color: var(--primary-color);
  box-shadow: 0 0 0 2px var(--primary-color-much-lighter);
}

.action-cell {
  width: 28px;
  text-align: center;
}

.remove-btn {
  background: none;
  border: none;
  cursor: pointer;
  font-size: 16px;
  color: #9ca3af;
  padding: 0 4px;
  line-height: 1;
}

.remove-btn:hover {
  color: #ef4444;
}

.unbound-section {
  margin-bottom: 12px;
}

.unbound-label {
  font-weight: 500;
  margin-bottom: 6px;
  color: #6b7280;
}

.unbound-row {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 4px;
}

.unbound-info {
  flex: 0 0 auto;
  font-size: 12px;
  white-space: nowrap;
}

.account-type {
  color: #9ca3af;
}

.unbound-row .affiliate-input {
  flex: 1 1 auto;
  min-width: 0;
}

.bind-btn {
  flex: 0 0 auto;
  background: var(--primary-color);
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 14px;
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  line-height: 1;
}

.bind-btn:hover:not(:disabled) {
  background: var(--primary-color-hover);
}

.bind-btn:disabled {
  opacity: 0.4;
  cursor: default;
}

.manual-add {
  margin-top: 8px;
  padding-top: 8px;
  border-top: 1px solid #e5e7eb;
}

.manual-add-label {
  font-weight: 500;
  margin-bottom: 6px;
  color: #6b7280;
}

.manual-add-row {
  display: flex;
  align-items: center;
  gap: 4px;
}

.add-select {
  flex: 0 0 auto;
  padding: 3px 4px;
  border: 1px solid #d1d5db;
  border-radius: 4px;
  font-size: 12px;
}

.add-account-input,
.add-affiliate-input {
  flex: 1 1 0;
  min-width: 0;
  padding: 3px 6px;
  border: 1px solid #d1d5db;
  border-radius: 4px;
  font-size: 12px;
}

.add-account-input:focus,
.add-affiliate-input:focus,
.add-select:focus {
  outline: none;
  border-color: var(--primary-color);
  box-shadow: 0 0 0 2px var(--primary-color-much-lighter);
}

.empty-hint {
  color: #9ca3af;
  font-size: 12px;
  line-height: 1.5;
}
</style>
