<script setup lang="ts">
import { ref, computed } from 'vue';
import { FileKind } from './file_manager_store.js';
import type { FileManagerState, FileEntry } from './file_manager_store.js';

const props = defineProps<{
   store: FileManagerState;
}>();

// --- UI state (local to this component) ---

const isExpanded = ref(false);
// null means "All"
const activeKindFilter = ref<FileKind | null>(null);
const lastClickedFilteredIndex = ref<number | null>(null);

// --- Constants ---

// Approximate rendered height of each table row (td padding 6px*2 + 13px font
// at ~1.4 line-height + 1px border). Used to pre-size the file list wrapper so
// the drawer height doesn't jump when the kind filter changes.
const ROW_HEIGHT_PX = 33;

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
const hasDownloadableSelected = computed(() =>
   selectedFiles.value.some((f) => f.isDownloadable)
);

// Height needed to show all files (unfiltered) plus the header row, capped at
// 50vh. Using CSS min() in an inline style keeps this stable across filter
// changes without needing a resize observer.
const fileListWrapperHeight = computed(() => {
   const totalPx = (props.store.files.length + 1) * ROW_HEIGHT_PX;
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

// --- Actions ---

function toggleExpanded() {
   isExpanded.value = !isExpanded.value;
   if (isExpanded.value) props.store.hasNotification = false;
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

function clearSelection() {
   props.store.files.forEach((f) => (f.isSelected = false));
   lastClickedFilteredIndex.value = null;
}
</script>

<template>
   <div class="fm-drawer" :class="{ 'fm-expanded': isExpanded }">

      <!-- Top bar -->
      <div class="fm-top-bar" @click="toggleExpanded">
         <div class="fm-top-bar-left">
            <span class="fm-title">Files</span>
            <span class="fm-count">({{ store.files.length }})</span>
            <span
               v-if="store.hasNotification"
               class="fm-notification-dot"
               title="Files were updated"
            ></span>
         </div>
         <button
            class="fm-toggle-btn"
            @click.stop="toggleExpanded"
            :aria-label="isExpanded ? 'Collapse' : 'Expand'"
         >
            <span class="fm-toggle-icon" :class="{ 'fm-rotated': isExpanded }">▲</span>
         </button>
      </div>

      <!-- Expandable content -->
      <div class="fm-content" v-show="isExpanded">

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
                  title="Clear selected"
                  @click="clearSelection"
               >
                  <img :src="'/images/bin_full.svg'" class="fm-action-icon">
               </button>
               <button
                  v-if="hasDownloadableSelected"
                  class="fm-action-btn"
                  title="Download selected"
               >&#x2B07;</button>
            </div>
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
                        </span>
                        <span
                           v-if="file.isDownloadable"
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

.fm-notification-dot {
   display: inline-block;
   width: 9px;
   height: 9px;
   border-radius: 50%;
   background-color: #ffcc00;
   box-shadow: 0 0 4px rgba(255, 204, 0, 0.7);
   margin-left: 2px;
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
   text-align: center;
   white-space: nowrap;
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
</style>
