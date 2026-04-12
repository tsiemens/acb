<template>
  <CollapsibleRegion>
    <template #header>Account Affiliates</template>
    <AccountBindingsEditor />
  </CollapsibleRegion>
</template>

<script lang="ts">
import { defineComponent } from 'vue';
import CollapsibleRegion from './CollapsibleRegion.vue';
import AccountBindingsEditor from './AccountBindingsEditor.vue';

export default defineComponent({
   name: 'BrokerConvertConfigPanel',
   components: { CollapsibleRegion, AccountBindingsEditor },
});
</script>
