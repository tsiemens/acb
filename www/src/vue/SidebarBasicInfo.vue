<template>
  <SidebarInfoSection class="info-warnings-section" v-show="store.warningsSectionVisible">
    <h3>Notices</h3>
    <p v-show="store.gitIssuesVisible">There are currently
      <a href="https://github.com/tsiemens/acb/issues?q=is%3Aissue+is%3Aopen+label%3A%22user+caveat%22">open caveat issues</a>.
      Please review them and ensure they do not impact your scenario.
    </p>

    <ErrorBox :store="gitErrorBoxStore" width="100%" />
  </SidebarInfoSection>

  <SidebarInfoSection>
    <p class="info-secondary">{{ versionText }}</p>
  </SidebarInfoSection>
</template>

<script lang="ts">
import { defineComponent, computed, type PropType } from 'vue';
import { webappVersion } from '../versions.js';
import type { SidebarInfoStore } from './sidebar_info_store.js';
import { ErrorBox as ErrorBoxModel, getErrorBoxStore } from './error_box_store.js';
import ErrorBox from './ErrorBox.vue';
import SidebarInfoSection from './SidebarInfoSection.vue';

export default defineComponent({
   name: 'SidebarBasicInfo',
   components: { ErrorBox, SidebarInfoSection },
   props: {
      store: {
         type: Object as PropType<SidebarInfoStore>,
         required: true,
      },
   },
   setup(props) {
      const gitErrorBoxStore = getErrorBoxStore(ErrorBoxModel.GIT_ERRORS_ID);

      const versionText = computed(() =>
         `acb ${props.store.acbVersion}, acb-web v${webappVersion}`
      );

      return { versionText, gitErrorBoxStore };
   },
});
</script>

<style scoped>
.info-warnings-section {
  background-color: var(--warning-bg);
  border: 1px solid var(--warning-border);
  padding: 15px;
  border-radius: var(--border-radius);
  margin-bottom: 20px;
}

.info-warnings-section h3 {
  color: var(--warning-text);
}

.info-secondary {
  color: var(--secondary-color);
}
</style>
