<template>
  <DialogShell
    :active="store.active"
    :title="store.title"
    :dismissOnBackdropClick="false"
    maxWidth="450px"
    @close="cancel"
  >
    <p>{{ store.message }}</p>

    <template #footer>
      <div class="confirm-dialog-actions">
        <button class="confirm-dialog-btn cancel" @click="cancel">{{ store.cancelLabel }}</button>
        <button class="confirm-dialog-btn confirm" @click="doConfirm">{{ store.confirmLabel }}</button>
      </div>
    </template>
  </DialogShell>
</template>

<script lang="ts">
import { defineComponent } from 'vue';
import { getConfirmDialogStore, resolveDialog } from './confirm_dialog_store.js';
import DialogShell from './DialogShell.vue';

export default defineComponent({
   name: 'ConfirmDialog',
   components: { DialogShell },
   setup() {
      const store = getConfirmDialogStore();

      function doConfirm() {
         resolveDialog(true);
      }
      function cancel() {
         resolveDialog(false);
      }

      return { store, doConfirm, cancel };
   },
});
</script>

<style scoped>
.confirm-dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}

.confirm-dialog-btn {
  padding: 8px 20px;
  border-radius: var(--border-radius);
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  border: 1px solid #ccc;
  transition: background-color 0.2s, border-color 0.2s;
}

.confirm-dialog-btn.cancel {
  background-color: #f5f5f5;
  color: #333;
}

.confirm-dialog-btn.cancel:hover {
  background-color: #e8e8e8;
}

.confirm-dialog-btn.confirm {
  background-color: var(--primary-color);
  color: white;
  border-color: var(--primary-color);
}

.confirm-dialog-btn.confirm:hover {
  opacity: 0.9;
}
</style>
