<template>
  <div
    class="dialog-backdrop"
    v-show="active"
    @click="onBackdropClick"
  ></div>

  <div
    class="dialog-frame"
    :class="{ active: active }"
    :style="{ maxWidth: maxWidth }"
  >
    <div class="dialog-header">
      <h3 class="dialog-title">{{ title }}</h3>
      <button class="dialog-close" @click="$emit('close')" title="Close - Esc">&times;</button>
    </div>
    <div class="dialog-content">
      <slot></slot>
    </div>
    <div v-if="$slots.footer" class="dialog-footer">
      <slot name="footer"></slot>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, watch, onUnmounted } from 'vue';

export default defineComponent({
   name: 'DialogShell',
   props: {
      active: {
         type: Boolean,
         required: true,
      },
      title: {
         type: String,
         required: true,
      },
      dismissOnBackdropClick: {
         type: Boolean,
         default: true,
      },
      maxWidth: {
         type: String,
         default: '600px',
      },
   },
   emits: ['close'],
   setup(props, { emit }) {
      function onBackdropClick() {
         if (props.dismissOnBackdropClick) {
            emit('close');
         }
      }

      function onKeyDown(e: KeyboardEvent) {
         if (e.key === 'Escape') {
            emit('close');
         }
      }

      watch(
         () => props.active,
         (active) => {
            if (active) {
               document.addEventListener('keydown', onKeyDown);
            } else {
               document.removeEventListener('keydown', onKeyDown);
            }
         }
      );

      onUnmounted(() => {
         document.removeEventListener('keydown', onKeyDown);
      });

      return { onBackdropClick };
   },
});
</script>

<style scoped>
.dialog-backdrop {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: rgba(0, 0, 0, 0.5);
  z-index: 100;
}

.dialog-frame {
  display: none;
  position: fixed;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  width: 90%;
  max-height: calc(100vh - 60px);
  background-color: white;
  border-radius: var(--border-radius);
  padding: 25px;
  box-shadow: 0 4px 15px rgba(0, 0, 0, 0.2);
  z-index: 110;
  flex-direction: column;
}

.dialog-frame.active {
  display: flex;
}

.dialog-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 15px;
  padding-bottom: 10px;
  border-bottom: 1px solid #eee;
}

.dialog-title {
  font-size: 20px;
  font-weight: 600;
  color: var(--primary-color);
}

.dialog-close {
  background: none;
  border: none;
  font-size: 20px;
  cursor: pointer;
  color: var(--secondary-color);
}

.dialog-content {
  line-height: 1.6;
  overflow-y: auto;
  min-height: 0;
}

.dialog-content :deep(p) {
  margin-bottom: 15px;
}
</style>
