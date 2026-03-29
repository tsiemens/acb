<script setup lang="ts">
import { ref } from 'vue';
// Special import for vite-svg-loader
import FolderIcon from '../../static/images/folder.svg?component';

const props = withDefaults(defineProps<{
   onFilesDropped: (fileList: FileList) => void;
   dropMessage?: string;
}>(), {
   dropMessage: 'Drop Files Here',
});


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
      <FolderIcon class="file-drop-icon" />
      <h3>{{ dropMessage }}</h3>
      <p>or click to browse files</p>
      <input
         ref="fileInput"
         type="file"
         multiple
         class="file-input-hidden"
         @change="handleFileInputChange"
      >
      <p class="local-disclaimer">All processing is done locally in your browser - no data leaves your computer</p>
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
   display: block;
   width: 40px;
   height: 40px;
   margin: 0 auto 10px;
   color: var(--secondary-color);
}

.file-input-hidden {
   display: none;
}

.local-disclaimer {
   margin-top: 12px;
   font-size: 0.85em;
   opacity: 0.5;
}
</style>
