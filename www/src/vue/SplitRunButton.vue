<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { AcbAppRunMode } from '../common/acb_app_types.js';
import { getTabStore, type TabIdType } from './tab_store.js';
import { getFileManagerStore, FileKind, relevantInputKindsForTab } from './file_manager_store.js';
import { AppFunctionMode } from '../common/acb_app_types.js';

interface RunModeOption {
   mode: AcbAppRunMode;
   label: string;
   icon?: string;
}

const props = defineProps<{
   tabId: TabIdType;
   onAction: (mode: AcbAppRunMode) => void;
   functionMode?: AppFunctionMode | null;
}>();

const menuOpen = ref(false);
const containerRef = ref<HTMLElement | null>(null);
const tabStore = getTabStore();
const fileManagerStore = getFileManagerStore();

const disabled = computed(() =>
   !(tabStore.runEnabledByTab.get(props.tabId) ?? false)
);

const BASE_RUN_MODE_OPTIONS: RunModeOption[] = [
   { mode: AcbAppRunMode.Run,    label: 'Run' },
   { mode: AcbAppRunMode.Export, label: 'Export', icon: '/images/submit-document.svg' },
];

const primaryOption = BASE_RUN_MODE_OPTIONS[0];

const dropdownOptions = computed<RunModeOption[]>(() => {
   const opts = BASE_RUN_MODE_OPTIONS.slice(1);
   if (props.functionMode === AppFunctionMode.Calculate) {
      opts.push({ mode: AcbAppRunMode.ExportTampermonkeyScript, label: 'Tampermonkey Script', icon: '/images/tampermonkey.svg' });
   }
   return opts;
});

const selectedFilesLabel = computed(() => {
   const kinds = relevantInputKindsForTab(props.tabId);
   const usedFiles = fileManagerStore.files.filter(f =>
      FileKind.isInput(f.kind) && f.useChecked && !f.warning && kinds.has(f.kind)
   );
   if (usedFiles.length === 0) return '';
   if (usedFiles.length === 1) return `${usedFiles[0].name} selected`;
   return `${usedFiles[0].name} and ${usedFiles.length - 1} other file${usedFiles.length - 1 > 1 ? 's' : ''} selected`;
});

function handlePrimaryClick() {
   if (disabled.value) return;
   props.onAction(primaryOption.mode);
}

function handleToggleClick() {
   if (disabled.value) return;
   menuOpen.value = !menuOpen.value;
}

function handleSelectMode(mode: AcbAppRunMode) {
   props.onAction(mode);
   menuOpen.value = false;
}

function handleClickOutside(event: MouseEvent) {
   if (containerRef.value && !containerRef.value.contains(event.target as Node)) {
      menuOpen.value = false;
   }
}

onMounted(() => document.addEventListener('click', handleClickOutside));
onUnmounted(() => document.removeEventListener('click', handleClickOutside));
</script>

<template>
   <div ref="containerRef" class="split-btn-container">
      <div class="split-btn-wrapper">
         <div class="split-btn" :class="{ disabled }">
            <button
               class="split-btn-primary"
               :disabled="disabled"
               @click="handlePrimaryClick"
            >
               {{ primaryOption.label }}
            </button>
            <button
               class="split-btn-toggle"
               :disabled="disabled"
               @click.stop="handleToggleClick"
               aria-label="Select action"
            >
               <span class="split-btn-arrow" :class="{ open: menuOpen }">&#x25BC;</span>
            </button>
         </div>
         <ul v-if="menuOpen" class="split-btn-menu">
            <li
               v-for="opt in dropdownOptions"
               :key="opt.mode"
               class="split-btn-menu-item"
               @click="handleSelectMode(opt.mode)"
            >
               <img v-if="opt.icon" :src="opt.icon" class="split-btn-menu-icon">
               {{ opt.label }}
            </li>
         </ul>
      </div>
      <span v-if="selectedFilesLabel" class="selected-files-label">{{ selectedFilesLabel }}</span>
   </div>
</template>

<style scoped>
.split-btn-container {
   display: inline-flex;
   align-items: center;
   gap: 10px;
}

.split-btn-wrapper {
   position: relative;
}

.selected-files-label {
   font-size: 13px;
   color: #9ca3af;
}

.split-btn {
   display: inline-flex;
   border-radius: 4px;
   overflow: hidden;
}

.split-btn-primary,
.split-btn-toggle {
   border: none;
   color: white;
   cursor: pointer;
   font-weight: 500;
   font-size: 14px;
   background-color: var(--primary-color);
   transition: background-color 0.15s;
}

.split-btn-primary {
   padding: 10px 20px;
}

.split-btn-toggle {
   padding: 10px 10px;
   border-left: 1px solid rgba(255, 255, 255, 0.3);
}

.split-btn-primary:hover:not(:disabled),
.split-btn-toggle:hover:not(:disabled) {
   background-color: var(--primary-color-hover);
}

.split-btn.disabled .split-btn-primary,
.split-btn.disabled .split-btn-toggle {
   filter: saturate(0.2) brightness(1.8);
   cursor: default;
}

.split-btn-arrow {
   display: inline-block;
   font-size: 10px;
   transition: transform 0.15s;
}

.split-btn-arrow.open {
   transform: rotate(180deg);
}

.split-btn-menu {
   position: absolute;
   top: 100%;
   left: 0;
   margin: 4px 0 0;
   padding: 4px 0;
   list-style: none;
   background: white;
   border: 1px solid #d1d5db;
   border-radius: 4px;
   box-shadow: 0 4px 12px rgba(0, 0, 0, 0.12);
   z-index: 10;
   min-width: 100%;
}

.split-btn-menu-item {
   display: flex;
   align-items: center;
   gap: 8px;
   padding: 8px 16px;
   cursor: pointer;
   font-size: 14px;
   color: #333;
   white-space: nowrap;
}

.split-btn-menu-icon {
   width: 16px;
   height: 16px;
   filter: invert(0.3);
}

.split-btn-menu-item:hover {
   background-color: #f0f4fa;
}
</style>
