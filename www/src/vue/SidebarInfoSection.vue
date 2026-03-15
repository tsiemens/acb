<template>
  <div class="info-section">
    <slot />
  </div>
</template>

<script lang="ts">
import { defineComponent } from 'vue';

export default defineComponent({
   name: 'SidebarInfoSection',
});
</script>

<style scoped>
.info-section {
  margin-bottom: 25px;
}

.info-section :deep(h3) {
  font-size: 18px;
  margin-bottom: 10px;
  color: var(--primary-color);
}

.info-section :deep(p) {
  margin-bottom: 10px;
}
</style>
