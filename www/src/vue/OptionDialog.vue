<template>
  <DialogShell
    :active="store.active"
    :title="store.title"
    :dismissOnBackdropClick="false"
    maxWidth="450px"
    @close="dismiss"
  >
    <p>{{ store.message }}</p>

    <template #footer>
      <div class="option-dialog-actions">
        <button
          v-for="opt in store.options"
          :key="opt.id"
          class="option-dialog-btn"
          :class="opt.affirmative ? 'affirmative' : 'cancel'"
          @click="choose(opt.id)"
        >{{ opt.text }}</button>
      </div>
    </template>
  </DialogShell>
</template>

<script lang="ts">
import { defineComponent } from 'vue';
import { getOptionDialogStore, resolveOptionDialog } from './option_dialog_store.js';
import DialogShell from './DialogShell.vue';

export default defineComponent({
   name: 'OptionDialog',
   components: { DialogShell },
   setup() {
      const store = getOptionDialogStore();

      function choose(id: string) {
         resolveOptionDialog(id);
      }
      function dismiss() {
         resolveOptionDialog(null);
      }

      return { store, choose, dismiss };
   },
});
</script>

<style scoped>
.option-dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  flex-wrap: wrap;
}

.option-dialog-btn {
  padding: 8px 20px;
  border-radius: var(--border-radius);
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  border: 1px solid #ccc;
  transition: background-color 0.2s, border-color 0.2s;
}

.option-dialog-btn.cancel {
  background-color: #f5f5f5;
  color: #333;
}

.option-dialog-btn.cancel:hover {
  background-color: #e8e8e8;
}

.option-dialog-btn.affirmative {
  background-color: var(--primary-color);
  color: white;
  border-color: var(--primary-color);
}

.option-dialog-btn.affirmative:hover {
  opacity: 0.9;
}
</style>
