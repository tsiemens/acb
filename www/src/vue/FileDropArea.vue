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
      class="file-drop-area"
      :class="{ 'drop-active': isDropActive }"
      @dragover="handleDragOver"
      @dragleave="handleDragLeave"
      @drop="handleDrop"
      @click="handleClick"
   >
      <div class="file-drop-icon">&#x1F4C1;</div>
      <h3>Drop CSV Files Here</h3>
      <p>or click to browse files</p>
      <input
         ref="fileInput"
         type="file"
         multiple
         class="file-input-hidden"
         @change="handleFileInputChange"
      >
   </div>
</template>

<style scoped>
.file-drop-area {
   border: 2px dashed #ccc;
   border-radius: var(--border-radius);
   padding: 25px;
   text-align: center;
   margin-bottom: 20px;
   transition: all 0.3s;
   cursor: pointer;
}

.file-drop-area:hover,
.file-drop-area.drop-active {
   border-color: var(--primary-color);
}

.file-drop-icon {
   font-size: 40px;
   margin-bottom: 10px;
   color: var(--secondary-color);
}

.file-input-hidden {
   display: none;
}
</style>
