<template>
  <div class="tab-content">
    <slot />
  </div>
</template>

<script lang="ts">
import { defineComponent } from 'vue';

export default defineComponent({
   name: 'TabContent',
   props: {
      tabId: {
         type: String,
         required: true,
      },
   },
});
</script>

<style scoped>
.tab-content {
  flex: 1;
  background-color: white;
  border-radius: var(--border-radius);
  padding: 20px;
  box-shadow: 0 2px 5px rgba(0,0,0,0.1);
}
</style>

<style>
.tab-content .content-separator {
  height: 2px;
  background: linear-gradient(90deg, transparent, #e2e8f0, transparent);
  margin: 30px 0;
}

.tab-content .files-and-buttons {
}

.tab-content .action-buttons {
  display: flex;
  gap: 10px;
  margin: 20px 0;
}
</style>
