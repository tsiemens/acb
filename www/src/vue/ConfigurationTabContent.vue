<template>
  <TabContent tabId="configuration">
    <div class="config-sections">

      <section class="config-section">
        <h3 class="section-heading">Account Bindings</h3>
        <p class="section-desc">
          Map broker account numbers to affiliate names. These apply during
          broker file conversion.
        </p>
        <AccountBindingsEditor />
      </section>

      <div class="section-divider"></div>

      <section class="config-section">
        <h3 class="section-heading">Symbol Renames</h3>
        <p class="section-desc">
          Rename ticker symbols before processing. Applies to both broker
          imports and ACB calculations. Chains are resolved recursively.
        </p>
        <SymbolRenamesEditor />
      </section>

    </div>
  </TabContent>
</template>

<script lang="ts">
import { defineComponent } from 'vue';
import TabContent from './TabContent.vue';
import AccountBindingsEditor from './AccountBindingsEditor.vue';
import SymbolRenamesEditor from './SymbolRenamesEditor.vue';

export default defineComponent({
   name: 'ConfigurationTabContent',
   components: { TabContent, AccountBindingsEditor, SymbolRenamesEditor },
});
</script>

<style scoped>
.config-sections {
  max-width: 640px;
}

.config-section {
  padding: 4px 0 16px;
}

.section-heading {
  margin: 0 0 4px 0;
  font-size: 15px;
  font-weight: 600;
  color: #374151;
}

.section-desc {
  margin: 0 0 12px 0;
  font-size: 13px;
  color: #6b7280;
  line-height: 1.5;
}

.section-divider {
  height: 1px;
  background: linear-gradient(90deg, transparent, #e2e8f0, transparent);
  margin: 8px 0 20px;
}
</style>
