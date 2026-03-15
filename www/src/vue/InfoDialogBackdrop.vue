<template>
  <div
    class="info-dialog-backdrop"
    v-show="store.activeDialogId !== null"
    @click="close"
  ></div>
</template>

<script lang="ts">
import { defineComponent, type PropType, watch, onUnmounted } from 'vue';
import { type InfoDialogStore, closeDialog } from './info_dialog_store.js';

export default defineComponent({
   name: 'InfoDialogBackdrop',
   props: {
      store: {
         type: Object as PropType<InfoDialogStore>,
         required: true,
      },
   },
   setup(props) {
      function close() {
         closeDialog();
      }

      function onKeyDown(e: KeyboardEvent) {
         if (e.key === 'Escape') {
            closeDialog();
         }
      }

      watch(
         () => props.store.activeDialogId,
         (id) => {
            if (id !== null) {
               document.addEventListener('keydown', onKeyDown);
            } else {
               document.removeEventListener('keydown', onKeyDown);
            }
         }
      );

      onUnmounted(() => {
         document.removeEventListener('keydown', onKeyDown);
      });

      return { close };
   },
});
</script>

<style scoped>
.info-dialog-backdrop {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: rgba(0, 0, 0, 0.5);
  z-index: 90;
}
</style>
