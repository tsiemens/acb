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

    <template #footer>
      <div class="tm-dialog-actions">
        <button class="tm-btn tm-btn-cancel" @click="dismiss">Cancel</button>
        <button class="tm-btn tm-btn-generate" @click="generate">Generate</button>
      </div>
    </template>
  </DialogShell>
</template>

<script lang="ts">
import { defineComponent } from 'vue';
import DialogShell from './DialogShell.vue';
import {
   getTampermonkeyDialogStore,
   resolveTampermonkeyDialog,
} from './tampermonkey_dialog_store.js';

export default defineComponent({
   name: 'TampermonkeyExportDialog',
   components: { DialogShell },
   setup() {
      const store = getTampermonkeyDialogStore();

      function generate() {
         resolveTampermonkeyDialog({
            year: store.selectedYear,
            affiliate: store.selectedAffiliate,
         });
      }

      function dismiss() {
         resolveTampermonkeyDialog(null);
      }

      return { store, generate, dismiss };
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
