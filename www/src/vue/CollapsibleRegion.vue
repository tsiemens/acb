<template>
  <div class="collapsible-wrapper">
    <button
      class="collapsible-content-btn"
      :class="{ active: expanded, expanded: expanded }"
      @click="toggle"
    >
      <img class="toggle-icon" :src="'/images/chevron-right.svg'"/>
      <slot name="header" />
    </button>
    <div
      class="collapsible-content"
      :class="expanded ? 'expanded' : 'collapsed'"
    >
      <div class="collapsible-content-inner">
        <div class="collapsible-content-padded-inner">
          <slot />
        </div>
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref } from 'vue';

export default defineComponent({
   name: 'CollapsibleRegion',
   setup() {
      const expanded = ref(false);

      function toggle() {
         expanded.value = !expanded.value;
      }

      return { expanded, toggle };
   },
});
</script>

<style scoped>
.collapsible-wrapper {
  margin: 20px 0;
  position: relative;
}

.collapsible-content-btn {
  background: none;
  border: none;
  cursor: pointer;
  padding: 8px 12px;
  border-radius: 6px;
  font-size: 14px;
  color: #6b7280;
  transition: all 0.2s ease;
  display: flex;
  align-items: center;
  gap: 6px;
  font-weight: 500;
}

.collapsible-content-btn:hover {
  background-color: #f3f4f6;
  color: #374151;
}

.toggle-icon {
  width: 16px;
  height: 16px;
  filter: invert(0.4);
  transition: transform 0.3s ease;
}

.collapsible-content-btn.expanded .toggle-icon {
  transform: rotate(90deg);
}

.collapsible-content {
  background: #f8fafc;
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  margin-top: 8px;
  overflow: hidden;
  transition: all 0.4s cubic-bezier(0.4, 0.0, 0.2, 1);
  box-shadow: inset 0 2px 8px rgba(0,0,0,0.06);
}

.collapsible-content.collapsed {
  max-height: 0;
  border-width: 0;
  margin-top: 0;
  opacity: 0;
}

.collapsible-content.expanded {
  max-height: 500px;
  opacity: 1;
}

.collapsible-content-inner {
  padding: 24px;
  background: linear-gradient(145deg, #f8fafc 0%, #f1f5f9 100%);
  position: relative;
}

.collapsible-content-inner::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 1px;
  background: linear-gradient(90deg, transparent, rgba(148, 163, 184, 0.3), transparent);
}

.collapsible-content-padded-inner {
  color: #475569;
  line-height: 1.6;
}

.collapsible-content-padded-inner :deep(h3) {
  margin: 0 0 16px 0;
  color: #334155;
  font-size: 18px;
  font-weight: 600;
}

.collapsible-content-padded-inner :deep(p) {
  margin: 0 0 12px 0;
}
</style>
