<template>
  <div
    class="info-dialog"
    :class="{ active: isActive }"
  >
    <div class="info-dialog-header">
      <h3 class="info-dialog-title">{{ title }}</h3>
      <button class="info-dialog-close" @click="close">&times;</button>
    </div>
    <div class="info-dialog-content">
      <slot></slot>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, computed, type PropType } from 'vue';
import { type InfoDialogStore, closeDialog } from './info_dialog_store.js';

export default defineComponent({
   name: 'InfoDialog',
   props: {
      store: {
         type: Object as PropType<InfoDialogStore>,
         required: true,
      },
      dialogId: {
         type: String,
         required: true,
      },
      title: {
         type: String,
         required: true,
      },
   },
   setup(props) {
      const isActive = computed(() => props.store.activeDialogId === props.dialogId);

      function close() {
         closeDialog();
      }

      return { isActive, close };
   },
});
</script>

<style scoped>
.info-dialog {
  display: none;
  position: fixed;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  width: 90%;
  max-width: 600px;
  background-color: white;
  border-radius: var(--border-radius);
  padding: 25px;
  box-shadow: 0 4px 15px rgba(0, 0, 0, 0.2);
  z-index: 100;
}

.info-dialog.active {
  display: block;
}

.info-dialog-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 15px;
  padding-bottom: 10px;
  border-bottom: 1px solid #eee;
}

.info-dialog-title {
  font-size: 20px;
  font-weight: 600;
  color: var(--primary-color);
}

.info-dialog-close {
  background: none;
  border: none;
  font-size: 20px;
  cursor: pointer;
  color: var(--secondary-color);
}

.info-dialog-content {
  line-height: 1.6;
}

.info-dialog-content :deep(p) {
  margin-bottom: 15px;
}
</style>
