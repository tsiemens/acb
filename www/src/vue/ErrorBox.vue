<script setup lang="ts">
import type { ErrorBoxState } from './error_box_store.js';
import WarningTriangleIcon from '../assets/warning-triangle.svg';

const props = defineProps<{
   store: ErrorBoxState;
   width?: string;
   severity?: 'error' | 'warning';
}>();

const sev = props.severity ?? 'error';
</script>

<template>
   <div class="error-container-outer">
      <div
         v-if="props.store.visible"
         class="error-container"
         :style="props.width ? { width: props.width } : {}"
      >
         <div :class="['error-header', `error-header--${sev}`]">
            <WarningTriangleIcon class="error-icon" />
            <span class="error-box-title">{{ props.store.title }}</span>
         </div>
         <div class="error-content">
            <p v-if="props.store.descPre" class="error-desc-pre">{{ props.store.descPre }}</p>
            <div v-if="props.store.errorText" :class="['error-message', `error-message--${sev}`]">{{ props.store.errorText }}</div>
            <p v-if="props.store.descPost" class="error-desc-post">{{ props.store.descPost }}</p>
         </div>
      </div>
   </div>
</template>

<style scoped>
.error-container-outer {
   width: 100%;
   display: flex;
   justify-content: center;
}

.error-container {
   width: 90%;
   max-width: 1000px;
   margin-bottom: 12px;
   background-color: #fff;
   border-radius: 8px;
   box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
   overflow: hidden;
}

.error-header {
   color: white;
   padding: 8px 10px;
   font-weight: 600;
   display: flex;
   align-items: center;
   gap: 10px; /* Gap between text and warning icon */
}

.error-header--error {
   background-color: #ff5252;
}

.error-header--warning {
   background-color: #f5a623;
}

.error-icon {
   width: 20px;
   height: 20px;
   flex-shrink: 0;
}

.error-content {
   padding: 12px 12px;
   color: #333;
   line-height: 1.5;
}

.error-message {
   margin-top: 10px;
   padding: 12px;
   background-color: #f8f8f8;
   font-family: monospace;
   word-break: break-word;
   white-space: pre-wrap;
}

.error-message--error {
   border-left: 4px solid #ff5252;
}

.error-message--warning {
   border-left: 4px solid #f5a623;
}

.error-desc-pre,
.error-desc-post {
   white-space: pre-wrap;
}

.error-desc-post {
   margin-top: 10px;
}
</style>
