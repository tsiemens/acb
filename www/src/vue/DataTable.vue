<template>
  <div>
    <div v-if="title" class="table-title">{{ title }}</div>

    <div v-if="errors.length > 0" class="security-errors">
      <p v-for="(err, i) in errors" :key="i">{{ err }}</p>
    </div>

    <div class="table-fixed-head">
      <table>
        <thead>
          <tr>
            <th v-for="(col, i) in table.header" :key="i">{{ col }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(row, ri) in table.rows" :key="ri">
            <td v-for="(cell, ci) in row" :key="ci">{{ cell }}</td>
          </tr>
        </tbody>
      </table>
    </div>

    <div v-if="notes.length > 0" class="security-notes">
      <p v-for="(note, i) in notes" :key="i">{{ note }}</p>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, computed, type PropType } from 'vue';
import type { RenderTable } from '../acb_wasm_types.js';

export default defineComponent({
   name: 'DataTable',
   props: {
      table: {
         type: Object as PropType<RenderTable>,
         required: true,
      },
      title: {
         type: String,
         default: '',
      },
   },
   setup(props) {
      const errors = computed(() => props.table.errors || []);
      const notes = computed(() => props.table.notes || []);
      return { errors, notes };
   },
});
</script>
