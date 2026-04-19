<template>
  <DialogShell
    :active="store.active"
    title="Export Tampermonkey Script"
    maxWidth="480px"
    @close="dismiss"
  >
    <p>
      Generates a
      <a href="https://www.tampermonkey.net/" target="_blank" rel="noopener">Tampermonkey</a>
      userscript that auto-fills your capital gains data into
      <strong>WealthSimple Tax</strong>.
    </p>
    <p>
      Select the tax year to export. Only dispositions from that year will be
      included in the generated script.
    </p>

    <div class="tm-field-row">
      <label for="tm-year-select" class="tm-field-label">Tax year:</label>
      <select
        id="tm-year-select"
        v-model="store.selectedYear"
        class="tm-field-select"
      >
        <option v-for="year in store.yearOptions" :key="year" :value="year">{{ year }}</option>
      </select>
    </div>

    <div v-if="store.affiliateOptions.length > 1" class="tm-field-row">
      <label for="tm-affiliate-select" class="tm-field-label">Affiliate:</label>
      <select
        id="tm-affiliate-select"
        v-model="store.selectedAffiliate"
        class="tm-field-select"
      >
        <option v-for="aff in store.affiliateOptions" :key="aff" :value="aff">{{ aff }}</option>
      </select>
    </div>

    <div v-if="filteredSecurities.length > 0" class="tm-securities-section">
      <div class="tm-securities-header">
        <span class="tm-field-label">Securities:</span>
        <span class="tm-securities-actions">
          <button type="button" class="tm-link-btn" @click="selectAllSecurities">All</button>
          <span class="tm-link-sep">|</span>
          <button type="button" class="tm-link-btn" @click="selectNoSecurities">None</button>
        </span>
      </div>
      <div class="tm-securities-list">
        <label v-for="sec in filteredSecurities" :key="sec" class="tm-security-item">
          <input
            type="checkbox"
            :checked="!!store.selectedSecurities[sec]"
            @change="toggleSecurity(sec, ($event.target as HTMLInputElement).checked)"
          />
          <span>{{ sec }}</span>
        </label>
      </div>
    </div>

    <div v-if="showUsdFxWarning" class="tm-warning-bubble">
      <strong>USD.FX</strong> net gain for the selected transactions is
      <strong>{{ usdFxGainDisplay }}</strong>, which is below the
      <strong class="tooltip tm-threshold-tip">${{ usdFxThreshold }}<span class="tooltiptext">
        Threshold value applicable to tax year {{ usdFxThresholdYear }}
        (at least). The CRA threshold may differ in other years — verify
        before relying on this.</span></strong> reporting threshold.
      You likely want to exclude <strong>USD.FX</strong> from the export.
    </div>

    <template #footer>
      <div class="tm-dialog-actions">
        <button class="tm-btn tm-btn-cancel" @click="dismiss">Cancel</button>
        <button class="tm-btn tm-btn-generate" @click="generate">Generate</button>
      </div>
    </template>
  </DialogShell>
</template>

<script lang="ts">
import { computed, defineComponent } from 'vue';
import DialogShell from './DialogShell.vue';
import {
   getTampermonkeyDialogStore,
   resolveTampermonkeyDialog,
} from './tampermonkey_dialog_store.js';
import { USD_FX_SECURITY } from '../render_table_utils.js';

const USD_FX_GAIN_WARNING_THRESHOLD_CAD = 200;
const USD_FX_GAIN_WARNING_THRESHOLD_YEAR = 2025;

export default defineComponent({
   name: 'TampermonkeyExportDialog',
   components: { DialogShell },
   setup() {
      const store = getTampermonkeyDialogStore();

      const filteredSecurities = computed(() => {
         const set = new Set<string>();
         for (const e of store.entries) {
            const entryYear = parseInt(e.settlementDate.split('-')[0], 10);
            if (entryYear !== store.selectedYear) continue;
            if (store.selectedAffiliate !== null
                && e.affiliate !== store.selectedAffiliate) continue;
            set.add(e.security);
         }
         const sorted = Array.from(set).sort();
         return sorted.includes(USD_FX_SECURITY)
            ? [USD_FX_SECURITY, ...sorted.filter((s) => s !== USD_FX_SECURITY)]
            : sorted;
      });

      const usdFxNetGain = computed(() => {
         let gain = 0;
         for (const e of store.entries) {
            if (e.security !== USD_FX_SECURITY) continue;
            const entryYear = parseInt(e.settlementDate.split('-')[0], 10);
            if (entryYear !== store.selectedYear) continue;
            if (store.selectedAffiliate !== null
                && e.affiliate !== store.selectedAffiliate) continue;
            gain += e.proceedsCad - e.costBaseCad - e.sellingExpensesCad;
         }
         return gain;
      });

      const showUsdFxWarning = computed(() => {
         if (!store.selectedSecurities[USD_FX_SECURITY]) return false;
         if (!filteredSecurities.value.includes(USD_FX_SECURITY)) return false;
         return usdFxNetGain.value < USD_FX_GAIN_WARNING_THRESHOLD_CAD;
      });

      const usdFxGainDisplay = computed(() => {
         const v = usdFxNetGain.value;
         const abs = Math.abs(v).toFixed(2);
         return v < 0 ? `-$${abs}` : `$${abs}`;
      });

      function generate() {
         const securities = filteredSecurities.value.filter(
            (sec) => store.selectedSecurities[sec]);
         resolveTampermonkeyDialog({
            year: store.selectedYear,
            affiliate: store.selectedAffiliate,
            securities,
         });
      }

      function dismiss() {
         resolveTampermonkeyDialog(null);
      }

      function toggleSecurity(sec: string, checked: boolean) {
         store.selectedSecurities[sec] = checked;
      }

      function selectAllSecurities() {
         for (const sec of filteredSecurities.value) {
            store.selectedSecurities[sec] = true;
         }
      }

      function selectNoSecurities() {
         for (const sec of filteredSecurities.value) {
            store.selectedSecurities[sec] = false;
         }
      }

      return {
         store,
         filteredSecurities,
         generate,
         dismiss,
         toggleSecurity,
         selectAllSecurities,
         selectNoSecurities,
         showUsdFxWarning,
         usdFxGainDisplay,
         usdFxThreshold: USD_FX_GAIN_WARNING_THRESHOLD_CAD,
         usdFxThresholdYear: USD_FX_GAIN_WARNING_THRESHOLD_YEAR,
      };
   },
});
</script>

<style scoped>
.tm-field-row {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: 6px;
}

.tm-field-label {
  font-size: 14px;
  white-space: nowrap;
}

.tm-field-select {
  font-size: 14px;
  padding: 4px 8px;
  border: 1px solid #ccc;
  border-radius: var(--border-radius);
}

.tm-securities-section {
  margin-top: 14px;
  margin-bottom: 12px;
}

.tm-securities-header {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  margin-bottom: 4px;
}

.tm-securities-actions {
  font-size: 13px;
}

.tm-link-btn {
  background: none;
  border: none;
  padding: 0;
  color: var(--primary-color);
  cursor: pointer;
  font-size: 13px;
}

.tm-link-btn:hover {
  text-decoration: underline;
}

.tm-link-sep {
  color: #999;
  margin: 0 6px;
}

.tm-securities-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
  max-height: 180px;
  overflow-y: auto;
  border: 1px solid #ccc;
  border-radius: var(--border-radius);
  padding: 8px 10px;
}

.tm-security-item {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  cursor: pointer;
}

.tm-warning-bubble {
  position: relative;
  margin-top: 12px;
  margin-bottom: 12px;
  padding: 10px 12px;
  background-color: #fff8d6;
  border: 1px solid #e6c200;
  border-radius: var(--border-radius);
  color: #5c4a00;
  font-size: 13px;
  line-height: 1.4;
}

.tm-warning-bubble :deep(.tooltip.tm-threshold-tip) {
  border-bottom-color: #5c4a00;
  cursor: help;
}

/* Anchor the tooltip to the bubble itself rather than the $200 span,
   so it cannot overflow the dialog regardless of where the span wraps. */
.tm-warning-bubble :deep(.tooltip.tm-threshold-tip .tooltiptext) {
  left: 0;
  right: 0;
  bottom: 100%;
  margin-bottom: 6px;
  width: auto;
}

.tm-dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}

.tm-btn {
  padding: 8px 20px;
  border-radius: var(--border-radius);
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  border: 1px solid #ccc;
  transition: background-color 0.2s, border-color 0.2s;
}

.tm-btn-cancel {
  background-color: #f5f5f5;
  color: #333;
}

.tm-btn-cancel:hover {
  background-color: #e8e8e8;
}

.tm-btn-generate {
  background-color: var(--primary-color);
  color: white;
  border-color: var(--primary-color);
}

.tm-btn-generate:hover {
  opacity: 0.9;
}
</style>
