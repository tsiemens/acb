<script setup lang="ts">
import { ref, computed } from 'vue';
import { FileKind } from './file_manager_store.js';
import type { FileManagerState, FileEntry } from './file_manager_store.js';
import { openDynamicTextDialog } from './info_dialog_store.js';

const props = defineProps<{
   store: FileManagerState;
   onFilesDropped?: (fileList: FileList) => void;
   onDownloadSelected?: (files: FileEntry[]) => void;
}>();

// --- UI state (local to this component) ---

// null means "All"
const activeKindFilter = ref<FileKind | null>(null);
const lastClickedFilteredIndex = ref<number | null>(null);
const showClearModal = ref(false);
const isDropActive = ref(false);
const fileInput = ref<HTMLInputElement | null>(null);

// --- Constants ---

// Approximate rendered height of each table row (td padding 6px*2 + 13px font
// at ~1.4 line-height + 1px border). Used to pre-size the file list wrapper so
// the drawer height doesn't jump when the kind filter changes.
const ROW_HEIGHT_PX = 34;

// --- Computed ---

const presentKinds = computed<FileKind[]>(() =>
   [...new Set(props.store.files.map((f) => f.kind))]
);

const filteredFiles = computed(() => {
   if (activeKindFilter.value === null) return props.store.files;
   return props.store.files.filter((f) => f.kind === activeKindFilter.value);
});

const selectedFiles = computed(() => filteredFiles.value.filter((f) => f.isSelected));
const hasSelection = computed(() => selectedFiles.value.length > 0);
const downloadableSelected = computed(() =>
   selectedFiles.value.filter((f) => f.isDownloadable)
);
const hasDownloadableSelected = computed(() => downloadableSelected.value.length > 0);

// Height needed to show all files (unfiltered) plus the header row, capped at
// 50vh. Using CSS min() in an inline style keeps this stable across filter
// changes without needing a resize observer.
const fileListWrapperHeight = computed(() => {
   const totalPx = (Math.max(props.store.files.length, 1) + 1) * ROW_HEIGHT_PX;
   return `min(${totalPx}px, 50vh)`;
});

const allVisibleInputUseChecked = computed({
   get() {
      const inputFiles = filteredFiles.value.filter((f) => FileKind.isInput(f.kind));
      return inputFiles.length > 0 && inputFiles.every((f) => f.useChecked);
   },
   set(val: boolean) {
      filteredFiles.value
         .filter((f) => FileKind.isInput(f.kind))
         .forEach((f) => (f.useChecked = val));
   },
});

const inputFiles = computed(() =>
   props.store.files.filter((f) => FileKind.isInput(f.kind))
);

const activeInputCount = computed(() =>
   inputFiles.value.filter(
      (f) => f.useChecked && !f.warning && props.store.relevantInputKinds.has(f.kind),
   ).length
);

const hasWarnings = computed(() => props.store.files.some((f) => f.warning));
const anyDetecting = computed(() => props.store.files.some((f) => f.isDetecting));

// --- Actions ---

function toggleExpanded() {
   props.store.isExpanded = !props.store.isExpanded;
   if (props.store.isExpanded) props.store.hasNotification = false;
}

function handleRowClick(file: FileEntry, index: number, event: MouseEvent) {
   if (event.shiftKey && lastClickedFilteredIndex.value !== null) {
      const start = Math.min(lastClickedFilteredIndex.value, index);
      const end = Math.max(lastClickedFilteredIndex.value, index);
      filteredFiles.value.slice(start, end + 1).forEach((f) => (f.isSelected = true));
   } else if (event.ctrlKey || event.metaKey) {
      file.isSelected = !file.isSelected;
   } else {
      const wasOnlySelected = selectedFiles.value.length === 1 && file.isSelected;
      props.store.files.forEach((f) => (f.isSelected = false));
      if (!wasOnlySelected) file.isSelected = true;
   }
   lastClickedFilteredIndex.value = index;
}

function selectAll() {
   filteredFiles.value.forEach((f) => (f.isSelected = true));
}

function handleClearClick() {
   if (hasDownloadableSelected.value) {
      showClearModal.value = true;
   } else {
      removeSelected();
   }
}

function handleDrawerDragOver(event: DragEvent) {
   event.preventDefault();
   if (event.dataTransfer) event.dataTransfer.dropEffect = 'copy';
   isDropActive.value = true;
}

function handleDrawerDragLeave() {
   isDropActive.value = false;
}

function handleDrawerDrop(event: DragEvent) {
   event.preventDefault();
   isDropActive.value = false;
   if (event.dataTransfer?.files && props.onFilesDropped) {
      props.onFilesDropped(event.dataTransfer.files);
   }
}

function handleDropZoneClick() {
   fileInput.value?.click();
}

function handleFileInputChange(event: Event) {
   const files = (event.target as HTMLInputElement).files;
   if (files && props.onFilesDropped) {
      props.onFilesDropped(files);
   }
   // Reset so the same file can be re-selected
   (event.target as HTMLInputElement).value = '';
}

function showExtractedText(file: FileEntry) {
   if (!file.pdfPageTexts) return;
   const content = file.pdfPageTexts
      .map((text, i) => `── Page ${i + 1} ──\n${text}`)
      .join('\n\n');
   openDynamicTextDialog(file.name, content);
}

function removeSelected() {
   props.store.removeFiles(selectedFiles.value.map((f) => f.id));
   lastClickedFilteredIndex.value = null;
   showClearModal.value = false;
}
</script>

<template>
   <div class="fm-drawer" :class="{ 'fm-expanded': store.isExpanded }">

      <!-- Top bar -->
      <div class="fm-top-bar" @click="toggleExpanded">
         <div class="fm-top-bar-left">
            <span class="fm-title">Files</span>
            <span class="fm-count">({{ store.files.length }})</span>
            <span
               v-if="activeInputCount !== store.files.length"
               class="fm-active-count"
            >· {{ activeInputCount }} active</span>
            <span
               v-if="hasWarnings"
               class="fm-top-bar-warning"
               title="One or more files have errors"
            >⚠</span>
            <span
               v-if="store.hasNotification"
               class="fm-notification-dot"
               title="Files were updated"
            ></span>
            <span v-if="anyDetecting" class="fm-scanning">
               <span class="fm-spinner"></span>
               Scanning files…
            </span>
         </div>
         <button
            class="fm-toggle-btn"
            @click.stop="toggleExpanded"
            :aria-label="store.isExpanded ? 'Collapse' : 'Expand'"
         >
            <span class="fm-toggle-icon" :class="{ 'fm-rotated': store.isExpanded }">▲</span>
         </button>
      </div>

      <!-- Confirmation modal -->
      <div v-if="showClearModal" class="fm-modal-overlay" @click.self="showClearModal = false">
         <div class="fm-modal">
            <p class="fm-modal-title">Clear generated files?</p>
            <p class="fm-modal-body">
               The following file<span v-if="downloadableSelected.length !== 1">s</span>
               will be permanently removed:
            </p>
            <ul class="fm-modal-file-list">
               <li v-for="f in downloadableSelected" :key="f.id">{{ f.name }}</li>
            </ul>
            <p v-if="selectedFiles.length > downloadableSelected.length" class="fm-modal-body">
               Plus {{ selectedFiles.length - downloadableSelected.length }} other selected
               file<span v-if="selectedFiles.length - downloadableSelected.length !== 1">s</span>.
            </p>
            <div class="fm-modal-actions">
               <button class="btn-modal-cancel" @click="showClearModal = false">Cancel</button>
               <button class="btn-modal-confirm" @click="removeSelected">Remove</button>
            </div>
         </div>
      </div>

      <!-- Expandable content -->
      <div class="fm-content" v-show="store.isExpanded">

         <!-- Action bar -->
         <div class="fm-action-bar">
            <div class="fm-kind-filters">
               <label
                  class="fm-kind-label"
                  :class="{ 'fm-kind-active': activeKindFilter === null }"
               >
                  <input type="radio" v-model="activeKindFilter" :value="null" class="fm-kind-radio">
                  All
               </label>
               <label
                  v-for="kind in presentKinds"
                  :key="kind"
                  class="fm-kind-label"
                  :class="{ 'fm-kind-active': activeKindFilter === kind }"
               >
                  <input type="radio" v-model="activeKindFilter" :value="kind" class="fm-kind-radio">
                  {{ FileKind.label(kind) }}
               </label>
            </div>
            <div class="fm-action-buttons">
               <button class="fm-action-btn" title="Select all" @click="selectAll">
                  <img :src="'/images/double_check.svg'" class="fm-action-icon">
               </button>
               <button
                  v-if="hasSelection"
                  class="fm-action-btn"
                  title="Remove selected"
                  @click="handleClearClick"
               >
                  <img :src="'/images/bin_full.svg'" class="fm-action-icon">
               </button>
               <button
                  v-if="hasDownloadableSelected && onDownloadSelected"
                  class="fm-action-btn"
                  title="Download selected"
                  @click="onDownloadSelected(downloadableSelected)"
               >&#x2B07;</button>
            </div>
         </div>

         <!-- Drop zone -->
         <div
            v-if="onFilesDropped"
            class="fm-drop-zone"
            :class="{ 'fm-drop-active': isDropActive }"
            @dragover="handleDrawerDragOver"
            @dragleave="handleDrawerDragLeave"
            @drop="handleDrawerDrop"
            @click="handleDropZoneClick"
         >
            Drop files here or&nbsp;<span class="fm-drop-browse">browse</span>
            <input
               ref="fileInput"
               type="file"
               multiple
               class="fm-drop-file-input"
               @change="handleFileInputChange"
            >
         </div>

         <!-- File list -->
         <div class="fm-file-list-wrapper" :style="{ height: fileListWrapperHeight }">
            <table class="fm-file-list">
               <thead>
                  <tr>
                     <th class="fm-col-use">
                        <span class="fm-col-use-label">Use</span>
                        <input
                           type="checkbox"
                           v-model="allVisibleInputUseChecked"
                           title="Check/uncheck all visible input files"
                        >
                     </th>
                     <th class="fm-col-name" title="File name"></th>
                     <th class="fm-col-tags" title="Tags"></th>
                  </tr>
               </thead>
               <tbody>
                  <tr v-if="filteredFiles.length === 0" class="fm-empty-row">
                     <td colspan="3">No files</td>
                  </tr>
                  <tr
                     v-for="(file, i) in filteredFiles"
                     :key="file.id"
                     class="fm-file-row"
                     :class="{
                        'fm-row-selected': file.isSelected,
                        'fm-row-warning': !!file.warning,
                     }"
                     @click="handleRowClick(file, i, $event)"
                  >
                     <td class="fm-col-use">
                        <input
                           v-if="FileKind.isInput(file.kind)"
                           type="checkbox"
                           v-model="file.useChecked"
                           :class="store.relevantInputKinds.has(file.kind)
                              ? 'fm-use-checkbox-relevant'
                              : 'fm-use-checkbox-neutral'"
                           @click.stop
                        >
                     </td>
                     <td class="fm-col-name">
                        {{ file.name }}
                        <span
                           v-if="file.warning"
                           class="fm-warning-icon"
                           :title="file.warning"
                        >⚠</span>
                     </td>
                     <td class="fm-col-tags">
                        <span v-if="activeKindFilter === null" class="fm-tag">
                           {{ FileKind.label(file.kind) }}
                           <span v-if="file.isDetecting" class="fm-detecting" title="Detecting file type...">...</span>
                        </span>
                        <span
                           v-if="file.pdfPageTexts"
                           class="fm-tag fm-tag-view-text"
                           title="View extracted text"
                           @click.stop="showExtractedText(file)"
                        >&#x1F4C4;</span>
                        <span
                           v-else-if="file.isDownloadable"
                           class="fm-tag fm-tag-download"
                           title="Select to download"
                        >&#x2B07;</span>
                     </td>
                  </tr>
               </tbody>
            </table>
         </div>

      </div>
   </div>
</template>

<style scoped>
.fm-drawer {
   position: fixed;
   bottom: 0;
   right: 0;
   width: max(25vw, 80ch);
   max-width: calc(100vw - 20px);
   background-color: #fff;
   border-radius: 8px 8px 0 0;
   box-shadow: 0 -3px 16px rgba(0, 0, 0, 0.18);
   z-index: 200;
   font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
   font-size: 14px;
   color: #333;
   user-select: none;
}

/* Top bar */

.fm-top-bar {
   display: flex;
   align-items: center;
   justify-content: space-between;
   padding: 10px 14px;
   cursor: pointer;
   border-radius: 8px 8px 0 0;
   background-color: var(--primary-color);
   color: white;
}

.fm-top-bar:hover {
   background-color: var(--primary-color-hover);
}

.fm-top-bar-left {
   display: flex;
   align-items: center;
   gap: 6px;
}

.fm-title {
   font-weight: 600;
   font-size: 15px;
}

.fm-count {
   font-size: 13px;
   opacity: 0.85;
}

.fm-active-count {
   font-size: 13px;
   opacity: 0.85;
}

.fm-top-bar-warning {
   font-size: 14px;
   color: #ffcc00;
   filter: drop-shadow(0 0 2px rgba(255, 204, 0, 0.6));
}

.fm-notification-dot {
   display: inline-block;
   width: 9px;
   height: 9px;
   border-radius: 50%;
   background-color: #ffcc00;
   box-shadow: 0 0 4px rgba(255, 204, 0, 0.7);
   margin-left: 2px;
}

.fm-scanning {
   display: inline-flex;
   align-items: center;
   gap: 4px;
   font-size: 11px;
   color: rgba(255, 255, 255, 0.75);
   margin-left: 6px;
}

.fm-spinner {
   display: inline-block;
   width: 10px;
   height: 10px;
   border: 1.5px solid rgba(255, 255, 255, 0.3);
   border-top-color: rgba(255, 255, 255, 0.85);
   border-radius: 50%;
   animation: fm-spin 0.8s linear infinite;
}

@keyframes fm-spin {
   to { transform: rotate(360deg); }
}

.fm-toggle-btn {
   background: none;
   border: none;
   color: white;
   cursor: pointer;
   padding: 2px 4px;
   font-size: 13px;
   line-height: 1;
}

.fm-toggle-icon {
   display: inline-block;
   transition: transform 0.2s ease;
}

.fm-toggle-icon.fm-rotated {
   transform: rotate(180deg);
}

/* Content */

.fm-content {
   border-top: 1px solid #e0e0e0;
}

/* Action bar */

.fm-action-bar {
   display: flex;
   align-items: flex-end;
   gap: 8px;
   padding: 8px 12px;
   border-bottom: 1px solid #ececec;
   background-color: var(--light-color);
}

.fm-kind-filters {
   display: flex;
   flex-wrap: wrap;
   gap: 4px;
   max-width: 70%;
}

.fm-kind-label {
   display: inline-flex;
   align-items: center;
   padding: 3px 10px;
   border: 1px solid #c8cdd3;
   border-radius: 12px;
   cursor: pointer;
   font-size: 12px;
   background-color: #fff;
   transition: background-color 0.15s, border-color 0.15s;
}

.fm-kind-label:hover {
   background-color: #e9ecef;
}

.fm-kind-label.fm-kind-active {
   background-color: var(--primary-color);
   border-color: var(--primary-color);
   color: #fff;
}

.fm-kind-radio {
   display: none;
}

.fm-action-buttons {
   margin-left: auto;
   display: flex;
   align-items: center;
   gap: 4px;
}

.fm-action-btn {
   background: none;
   border: 1px solid #c8cdd3;
   border-radius: 4px;
   padding: 3px 8px;
   cursor: pointer;
   font-size: 15px;
   color: #555;
   line-height: 1;
   display: flex;
   align-items: center;
   transition: background-color 0.15s;
}

.fm-action-btn:hover {
   background-color: #e9ecef;
}

.fm-action-icon {
   width: 16px;
   height: 16px;
   display: block;
   filter: invert(0.4);
}

/* Drop zone */

.fm-drop-zone {
   margin: 8px 12px;
   padding: 18px 12px;
   min-height: 60px;
   display: flex;
   align-items: center;
   justify-content: center;
   border: 2px dashed #c8cdd3;
   border-radius: 6px;
   text-align: center;
   font-size: 12px;
   color: #888;
   cursor: pointer;
   transition: border-color 0.15s, background-color 0.15s, color 0.15s;
}

.fm-drop-zone:hover {
   border-color: #999;
   background-color: #fafafa;
}

.fm-drop-zone.fm-drop-active {
   border-color: var(--primary-color);
   background-color: var(--light-color);
   color: var(--primary-color);
}

.fm-drop-browse {
   text-decoration: underline;
   color: var(--primary-color);
}

.fm-drop-file-input {
   display: none;
}

/* File list */

.fm-file-list-wrapper {
   overflow-y: auto;
}

.fm-file-list {
   width: 100%;
   border-collapse: collapse;
}

.fm-file-list thead tr {
   background-color: #f0f2f5;
   position: sticky;
   top: 0;
   z-index: 1;
}

.fm-file-list th {
   padding: 6px 10px;
   font-weight: 600;
   font-size: 12px;
   color: #555;
   border-bottom: 1px solid #ddd;
   text-align: left;
}

.fm-col-use {
   width: 60px;
   white-space: nowrap;
}

th.fm-col-use {
   text-align: center;
}

/* Select every direct child of a th.fm-col-use */
th.fm-col-use > * {
   display: inline-flex;
   align-items: center;
   vertical-align: middle;
}

.fm-col-use-label {
   margin-right: 4px;
   font-size: 12px;
}

.fm-col-name {
   /* takes remaining space */
}

.fm-col-tags {
   width: 1%;
   white-space: nowrap;
   text-align: right;
   padding-right: 12px;
}

.fm-empty-row td {
   padding: 10px;
   text-align: center;
   color: #aaa;
   font-style: italic;
   font-size: 12px;
}

.fm-file-row {
   cursor: pointer;
   transition: background-color 0.1s;
}

.fm-file-row:hover {
   background-color: #f0f4fa;
}

.fm-file-row.fm-row-selected {
   background-color: #d6e4f7;
}

.fm-file-row.fm-row-selected:hover {
   background-color: #c4d9f5;
}

.fm-file-row.fm-row-warning td {
   background-color: #fff8e1;
}

.fm-file-row.fm-row-warning:hover td {
   background-color: #fff0c0;
}

.fm-file-row.fm-row-warning.fm-row-selected td {
   background-color: #fce8b2;
}

.fm-file-list td {
   padding: 6px 10px;
   border-bottom: 1px solid #f0f0f0;
   font-size: 13px;
}

.fm-use-checkbox-relevant,
.fm-use-checkbox-neutral {
   cursor: pointer;
}

.fm-use-checkbox-relevant {
   accent-color: var(--primary-color);
}

.fm-use-checkbox-neutral {
   accent-color: #929292;
}

.fm-warning-icon {
   margin-left: 5px;
   color: #c47a00;
   font-size: 12px;
   cursor: default;
}

.fm-tag {
   display: inline-block;
   padding: 1px 7px;
   border-radius: 10px;
   font-size: 11px;
   background-color: #e9ecef;
   color: #555;
   margin-left: 4px;
}

.fm-tag-download {
   background-color: #d4edda;
   color: #155724;
}

.fm-tag-view-text {
   background-color: #d6e4f7;
   color: #1a4a7a;
   cursor: pointer;
}

.fm-tag-view-text:hover {
   background-color: #c0d4ef;
}

/* Confirmation modal */

.fm-modal-overlay {
   position: absolute;
   inset: 0;
   background-color: rgba(0, 0, 0, 0.35);
   border-radius: 8px 8px 0 0;
   display: flex;
   align-items: center;
   justify-content: center;
   z-index: 10;
}

.fm-modal {
   background: #fff;
   border-radius: 8px;
   box-shadow: 0 4px 20px rgba(0, 0, 0, 0.22);
   padding: 20px 24px 16px;
   width: min(340px, 90%);
}

.fm-modal-title {
   font-weight: 600;
   font-size: 15px;
   margin: 0 0 10px;
}

.fm-modal-body {
   font-size: 13px;
   color: #444;
   margin: 0 0 6px;
}

.fm-modal-file-list {
   font-size: 13px;
   color: #333;
   margin: 0 0 10px;
   padding-left: 18px;
}

.fm-modal-file-list li {
   margin-bottom: 2px;
   word-break: break-all;
}

.fm-modal-actions {
   display: flex;
   justify-content: flex-end;
   gap: 8px;
   margin-top: 14px;
}

.btn-modal-cancel,
.btn-modal-confirm {
   padding: 5px 16px;
   border-radius: 4px;
   font-size: 13px;
   cursor: pointer;
   border: 1px solid transparent;
}

.btn-modal-cancel {
   background: #f0f0f0;
   border-color: #ccc;
   color: #333;
}

.btn-modal-cancel:hover {
   background: #e4e4e4;
}

.btn-modal-confirm {
   background: #c0392b;
   color: #fff;
}

.btn-modal-confirm:hover {
   background: #a93226;
}
</style>
