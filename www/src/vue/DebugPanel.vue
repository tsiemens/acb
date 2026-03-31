<template>
  <div v-if="isDebugMode" class="user-actions">
    <button class="btn btn-secondary" @click="panelVisible = !panelVisible">Debug Settings</button>

    <div v-show="panelVisible" class="debug-panel">
      <h4>Debug Settings</h4>
      <div class="debug-section">
        <button class="btn btn-primary btn-debug" @click="generateGithubOpenIssues">Generate GitHub Open Issues Warning</button>
        <button class="btn btn-primary btn-debug" @click="generateGithubFetchError">Generate GitHub Fetch Error</button>
      </div>

      <div class="debug-section">
        <div class="debug-control-row">
          <label class="debug-label">Sample Set:</label>
          <select :value="selectedSample" @change="onSampleChange" class="debug-select">
            <option value="none">— None —</option>
            <option v-for="set in manifestSets" :key="set.id" :value="set.id">{{ set.label }}</option>
            <option value="all">All</option>
          </select>
        </div>

        <div class="debug-control-row">
          <label class="debug-label">Auto-load mode:</label>
          <select :value="autoloadMode" @change="onAutoloadModeChange" class="debug-select">
            <option value="none">None</option>
            <option value="load">Load Only</option>
            <option value="run">Load and Run</option>
          </select>
        </div>

        <button
          class="btn btn-primary btn-debug"
          :disabled="selectedSample === 'none'"
          @click="loadNow"
        >Load Now</button>
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref } from 'vue';
import { handleGitUserCaveatIssues } from '../github.js';
import { isDebugModeEnabled } from '../debug.js';
import { loadManifest, loadSampleSet, autoRunHandler, type ManifestSet } from '../app_debug.js';

type AutoloadMode = 'none' | 'load' | 'run';

function getUrlParam(name: string): string {
   return new URLSearchParams(window.location.search).get(name) ?? '';
}

export default defineComponent({
   name: 'DebugPanel',
   setup() {
      const isDebugMode = isDebugModeEnabled();
      const panelVisible = ref(false);
      const manifestSets = ref<ManifestSet[]>([]);

      const selectedSample = ref<string>(getUrlParam('debug_sample') || 'none');
      const autoloadMode = ref<AutoloadMode>(
         (getUrlParam('debug_autoload') as AutoloadMode) || 'none'
      );

      // Fetch manifest to populate the dropdown.
      if (isDebugMode) {
         loadManifest().then((m) => {
            manifestSets.value = m.sets;
         }).catch((err: unknown) => {
            console.warn('DebugPanel: failed to load manifest:', err);
         });
      }

      function generateGithubOpenIssues() {
         handleGitUserCaveatIssues(["some caveat"]);
      }

      function generateGithubFetchError() {
         const date = new Date();
         if ((date.getSeconds() % 2) === 0) {
            handleGitUserCaveatIssues({"message": "Sample issue from github"});
         } else {
            handleGitUserCaveatIssues({"bla": "bar"});
         }
      }

      function onSampleChange(event: Event) {
         const val = (event.target as HTMLSelectElement).value;
         selectedSample.value = val;
         const url = new URL(window.location.href);
         if (val === 'none') {
            url.searchParams.delete('debug_sample');
         } else {
            url.searchParams.set('debug_sample', val);
         }
         history.replaceState(null, '', url.toString());
      }

      function onAutoloadModeChange(event: Event) {
         const val = (event.target as HTMLSelectElement).value as AutoloadMode;
         const url = new URL(window.location.href);
         if (val === 'none') {
            url.searchParams.delete('debug_autoload');
         } else {
            url.searchParams.set('debug_autoload', val);
         }
         window.location.href = url.toString();
      }

      function loadNow() {
         const setId = selectedSample.value;
         if (setId === 'none') return;
         loadSampleSet(setId).catch((err: unknown) => {
            console.error('DebugPanel loadNow failed:', err);
         });
      }

      // On page load: trigger auto-load if configured.
      if (isDebugMode) {
         const mode = autoloadMode.value;
         const setId = selectedSample.value;
         if ((mode === 'load' || mode === 'run') && setId && setId !== 'none') {
            if (mode === 'run') {
               autoRunHandler(setId).catch((err: unknown) => {
                  console.error('DebugPanel autoRunHandler failed:', err);
               });
            } else {
               loadSampleSet(setId).catch((err: unknown) => {
                  console.error('DebugPanel loadSampleSet failed:', err);
               });
            }
         }
      }

      return {
         isDebugMode, panelVisible, manifestSets, selectedSample, autoloadMode,
         generateGithubOpenIssues, generateGithubFetchError,
         onSampleChange, onAutoloadModeChange, loadNow,
      };
   },
});
</script>

<style scoped>
.debug-panel {
  position: absolute;
  right: 0;
  border: 1px solid #ccc;
  padding: 10px;
  margin-top: 5px;
  background-color: #f9f9f9;
  z-index: 1;
  width: max-content;
}

.debug-section {
  border: 1px solid #ddd;
  border-radius: 4px;
  padding: 8px;
  margin-top: 8px;
}

.btn-debug {
  margin-bottom: 5px;
  width: 100%;
}

.debug-control-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;
}

.debug-label {
  flex: 0 0 auto;
  font-size: 13px;
  white-space: nowrap;
}

.debug-select {
  flex: 1 1 auto;
  font-size: 13px;
  padding: 2px 4px;
}
</style>
