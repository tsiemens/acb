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

<style scoped>
header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 30px;
}

.header-content {
  display: flex;
  align-items: center;
  gap: 10px; /* Optional: Adjust spacing between the logo and the title */
}

.logo {
  width: 50px;
  height: auto;
}

.nav-links {
  display: flex;
  gap: 20px;
}

.nav-link {
  color: var(--primary-color);
  text-decoration: none;
  font-weight: 500;
  padding: 5px 0;
  position: relative;
}

.nav-link:hover {
  color: var(--primary-color-hover);
}

.nav-link:after {
  content: '';
  position: absolute;
  width: 0;
  height: 2px;
  bottom: 0;
  left: 0;
  background-color: var(--primary-color);
  transition: width 0.3s;
}

.nav-link:hover:after {
  width: 100%;
}

.app-title {
  font-size: 24px;
  font-weight: 600;
  color: var(--dark-color);
}
</style>
