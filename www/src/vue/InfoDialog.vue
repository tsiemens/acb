<template>
  <DialogShell
    :active="isActive"
    :title="title"
    :max-width="maxWidth"
    @close="close"
  >
    <slot></slot>
  </DialogShell>
</template>

<script lang="ts">
import { defineComponent, computed, type PropType } from 'vue';
import { type InfoDialogStore, closeDialog } from './info_dialog_store.js';
import DialogShell from './DialogShell.vue';

export default defineComponent({
   name: 'InfoDialog',
   components: { DialogShell },
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
      maxWidth: {
         type: String,
         default: '600px',
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
