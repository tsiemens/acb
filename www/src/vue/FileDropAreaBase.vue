<script setup lang="ts">
import { ref } from 'vue';

const props = defineProps<{
   onFilesDropped: (fileList: FileList) => void;
}>();

const isDropActive = ref(false);
const fileInput = ref<HTMLInputElement | null>(null);

function handleDragOver(event: DragEvent) {
   event.stopPropagation();
   event.preventDefault();
   if (event.dataTransfer) {
      event.dataTransfer.dropEffect = 'copy';
   }
   isDropActive.value = true;
}

function handleDragLeave() {
   isDropActive.value = false;
}

function handleDrop(event: DragEvent) {
   event.stopPropagation();
   event.preventDefault();
   isDropActive.value = false;
   if (event.dataTransfer?.files) {
      props.onFilesDropped(event.dataTransfer.files);
   }
}

function handleClick() {
   fileInput.value?.click();
}

function handleFileInputChange(event: Event) {
   const files = (event.target as HTMLInputElement).files;
   if (files) {
      props.onFilesDropped(files);
   }
   // Reset so the same file can be re-selected
   (event.target as HTMLInputElement).value = '';
}
</script>

<template>
   <div
      class="drop-zone"
      :class="{ 'drop-zone-active': isDropActive }"
      @dragover="handleDragOver"
      @dragleave="handleDragLeave"
      @drop="handleDrop"
      @click="handleClick"
   >
      <slot :isDropActive="isDropActive" />
      <input
         ref="fileInput"
         type="file"
         multiple
         class="drop-zone-file-input"
         @change="handleFileInputChange"
      >
   </div>
</template>

<style scoped>
.drop-zone {
   border: 2px dashed #c8cdd3;
   border-radius: var(--border-radius);
   text-align: center;
   cursor: pointer;
   transition: border-color 0.2s, background-color 0.2s;
}

.drop-zone:hover {
   border-color: #999;
   background-color: #f7f7f7;
}

.drop-zone.drop-zone-active {
   border-color: var(--primary-color);
   background-color: #e8f0fc;
}

.drop-zone-file-input {
   display: none;
}
</style>
