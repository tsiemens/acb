<template>
  <div
    class="info-dialog-backdrop"
    v-show="store.activeDialogId !== null"
    @click="close"
  ></div>
</template>

<script lang="ts">
import { defineComponent, type PropType } from 'vue';
import { type InfoDialogStore, closeDialog } from './info_dialog_store.js';

export default defineComponent({
   name: 'InfoDialogBackdrop',
   props: {
      store: {
         type: Object as PropType<InfoDialogStore>,
         required: true,
      },
   },
   setup() {
      function close() {
         closeDialog();
      }

      return { close };
   },
});
</script>
