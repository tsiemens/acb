<template>
  <header>
    <div class="header-content">
      <img :src="'/images/acb_logo.svg'" alt="ACB Logo" class="logo">
      <h1 class="app-title">ACB - A Capital Gains Calculator For Canadians</h1>
    </div>
    <div class="nav-links">
      <a href="https://github.com/tsiemens/acb/wiki" class="nav-link">Documentation</a>
      <a href="https://github.com/tsiemens/acb/wiki/FAQ" class="nav-link">FAQ</a>

      <DebugPanel :onAutoRun="onAutoRun" />
    </div>
  </header>
</template>

<script lang="ts">
import { defineComponent, type PropType } from 'vue';
import DebugPanel from './DebugPanel.vue';

export default defineComponent({
   name: 'AppHeader',
   components: { DebugPanel },
   props: {
      onAutoRun: {
         type: Function as PropType<() => void>,
         required: false,
      },
   },
});
</script>
