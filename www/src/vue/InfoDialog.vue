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
