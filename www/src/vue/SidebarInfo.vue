<template>
  <div class="info-section info-warnings-section" v-show="store.warningsSectionVisible">
    <h3>Notices</h3>
    <p v-show="store.gitIssuesVisible">There are currently
      <a href="https://github.com/tsiemens/acb/issues?q=is%3Aissue+is%3Aopen+label%3A%22user+caveat%22">open caveat issues</a>.
      Please review them and ensure they do not impact your scenario.
    </p>

    <div :id="gitErrorsId"></div>
  </div>

  <div class="info-section">
    <p class="info-secondary">{{ versionText }}</p>
  </div>
</template>

<script lang="ts">
import { defineComponent, computed, type PropType } from 'vue';
import { webappVersion } from '../versions.js';
import type { SidebarInfoStore } from './sidebar_info_store.js';
import { ErrorBox } from '../ui_model/error_displays.js';

export default defineComponent({
   name: 'SidebarInfo',
   props: {
      store: {
         type: Object as PropType<SidebarInfoStore>,
         required: true,
      },
   },
   setup(props) {
      const gitErrorsId = ErrorBox.GIT_ERRORS_ID;

      const versionText = computed(() =>
         `acb ${props.store.acbVersion}, acb-web v${webappVersion}`
      );

      return { versionText, gitErrorsId };
   },
});
</script>
