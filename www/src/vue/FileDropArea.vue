<script setup lang="ts">
// Special import for vite-svg-loader
import FolderIcon from '../../static/images/folder.svg?component';
import FileDropAreaBase from './FileDropAreaBase.vue';

withDefaults(defineProps<{
   onFilesDropped: (fileList: FileList) => void;
   dropMessage?: string;
}>(), {
   dropMessage: 'Drop Files Here',
});
</script>

<template>
   <FileDropAreaBase
      class="file-drop-area"
      :onFilesDropped="onFilesDropped"
   >
      <FolderIcon class="file-drop-icon" />
      <h3>{{ dropMessage }}</h3>
      <p>or click to browse files</p>
      <p class="local-disclaimer">All processing is done locally in your browser - no data leaves your computer</p>
   </FileDropAreaBase>
</template>

<style scoped>
.file-drop-area {
   padding: 25px;
   margin-bottom: 20px;
}

.file-drop-icon {
   display: block;
   width: 40px;
   height: 40px;
   margin: 0 auto 10px;
   color: var(--secondary-color);
}

.local-disclaimer {
   margin-top: 12px;
   font-size: 0.85em;
   opacity: 0.5;
}
</style>
