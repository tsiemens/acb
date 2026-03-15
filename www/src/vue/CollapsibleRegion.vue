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
