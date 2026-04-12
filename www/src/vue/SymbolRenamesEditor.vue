<template>
  <div class="config-panel">
    <!-- Existing renames -->
    <div v-if="allRenames.length > 0" class="renames-section">
      <table class="renames-table">
        <thead>
          <tr>
            <th>From</th>
            <th>To</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="r in allRenames" :key="r.from">
            <td class="symbol-cell from-cell">{{ r.from }}</td>
            <td class="symbol-cell to-cell">
              <input
                type="text"
                class="symbol-input"
                :value="r.to"
                @change="onEditRename(r.from, ($event.target as HTMLInputElement).value)"
                placeholder="Symbol"
              >
            </td>
            <td class="action-cell">
              <button
                class="remove-btn"
                title="Remove rename"
                @click="onRemoveRename(r.from)"
              >&times;</button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Add form -->
    <div class="manual-add">
      <div class="manual-add-label">Add rename:</div>
      <div class="manual-add-row">
        <input
          type="text"
          v-model="addFrom"
          placeholder="From symbol"
          class="add-symbol-input"
          @input="onAddInputChange"
        >
        <span class="arrow-label">&rarr;</span>
        <input
          type="text"
          v-model="addTo"
          placeholder="To symbol"
          class="add-symbol-input"
          @input="onAddInputChange"
        >
        <button
          class="bind-btn"
          :disabled="!addFrom.trim() || !addTo.trim()"
          @click="onManualAdd"
        >+</button>
      </div>
      <div v-if="addWarning" class="add-warning">{{ addWarning }}</div>
    </div>

    <div v-if="allRenames.length === 0" class="empty-hint">
      No symbol renames configured. Use this to map inconsistent ticker symbols
      to a canonical form (e.g. XEQT &rarr; XEQT.TO). Renames apply to both
      broker imports and ACB calculations.
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref, computed } from 'vue';
import {
   getConfigStore, setSymbolRename, removeSymbolRename,
} from './config_store.js';

interface Rename {
   from: string;
   to: string;
}

export default defineComponent({
   name: 'SymbolRenamesEditor',
   setup() {
      const configStore = getConfigStore();

      const addFrom = ref('');
      const addTo = ref('');
      const addWarning = ref('');

      const allRenames = computed<Rename[]>(() => {
         const renames = configStore.config?.symbol_renames ?? {};
         return Object.entries(renames).map(([from, to]) => ({ from, to }));
      });

      function onAddInputChange() {
         const from = addFrom.value.trim();
         const to = addTo.value.trim();
         if (!from || !to) {
            addWarning.value = '';
            return;
         }
         const renames = configStore.config?.symbol_renames ?? {};
         if (from === to) {
            addWarning.value = 'Warning: "from" and "to" are the same symbol.';
         } else if (from in renames) {
            addWarning.value = `Warning: "${from}" already has a rename. Adding will overwrite it.`;
         } else if (to in renames) {
            addWarning.value = `Warning: "${to}" is already a "from" key — chaining renames is not supported.`;
         } else {
            addWarning.value = '';
         }
      }

      function onEditRename(from: string, newTo: string) {
         const to = newTo.trim();
         if (!to) {
            onRemoveRename(from);
            return;
         }
         setSymbolRename(configStore, from, to);
      }

      function onRemoveRename(from: string) {
         removeSymbolRename(configStore, from);
      }

      function onManualAdd() {
         const from = addFrom.value.trim();
         const to = addTo.value.trim();
         if (!from || !to) return;
         setSymbolRename(configStore, from, to);
         addFrom.value = '';
         addTo.value = '';
         addWarning.value = '';
      }

      return {
         allRenames,
         addFrom,
         addTo,
         addWarning,
         onAddInputChange,
         onEditRename,
         onRemoveRename,
         onManualAdd,
      };
   },
});
</script>

<style scoped>
.config-panel {
  font-size: 13px;
}

.renames-section {
  margin-bottom: 12px;
}

.renames-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}

.renames-table th {
  text-align: left;
  font-weight: 600;
  padding: 4px 6px;
  border-bottom: 1px solid #d1d5db;
  color: #374151;
}

.renames-table td {
  padding: 4px 6px;
  border-bottom: 1px solid #e5e7eb;
}

.symbol-cell {
  font-family: monospace;
  font-size: 12px;
}

.from-cell {
  white-space: nowrap;
  color: #6b7280;
}

.symbol-input {
  width: 100%;
  padding: 3px 6px;
  border: 1px solid #d1d5db;
  border-radius: 4px;
  font-size: 12px;
  font-family: monospace;
}

.symbol-input:focus {
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

.add-symbol-input {
  flex: 1 1 0;
  min-width: 0;
  padding: 3px 6px;
  border: 1px solid #d1d5db;
  border-radius: 4px;
  font-size: 12px;
  font-family: monospace;
}

.add-symbol-input:focus {
  outline: none;
  border-color: var(--primary-color);
  box-shadow: 0 0 0 2px var(--primary-color-much-lighter);
}

.arrow-label {
  flex: 0 0 auto;
  color: #6b7280;
  font-size: 14px;
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

.add-warning {
  margin-top: 4px;
  font-size: 12px;
  color: #b45309;
}

.empty-hint {
  color: #9ca3af;
  font-size: 12px;
  line-height: 1.5;
}
</style>
