<template>
  <div>
    <div v-if="title" class="table-title">{{ title }}</div>

    <div v-if="errors.length > 0" class="security-errors">
      <p v-for="(err, i) in errors" :key="i">{{ err }}</p>
      <p v-if="errorSuffix">{{ errorSuffix }}</p>
    </div>

    <div class="table-fixed-head">
      <table>
        <thead>
          <tr>
            <th v-for="(col, i) in table.header" :key="i">{{ col }}</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="(row, ri) in table.rows"
            :key="ri"
            :class="rowClassFn ? rowClassFn(row) : ''"
          >
            <td v-for="(cell, ci) in row" :key="ci">{{ cell }}</td>
          </tr>
          <tr v-if="table.footer">
            <td v-for="(cell, ci) in table.footer" :key="ci">{{ cell }}</td>
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
      errorSuffix: {
         type: String,
         default: '',
      },
      rowClassFn: {
         type: Function as PropType<(row: string[]) => string | string[]>,
         default: null,
      },
   },
   setup(props) {
      const errors = computed(() => props.table.errors || []);
      const notes = computed(() => props.table.notes || []);
      return { errors, notes };
   },
});
</script>

<style scoped>
.table-title {
  padding-left: 8px;
  background: var(--dark-color);
  color: white;
  border-radius: 5px;
  margin-bottom: 5px;
  margin-top: 20px;
  font-weight: bold;
}

.security-errors {
  border: 1px solid #f5c6cb;
  background: #fbe6e7;
  border-radius: 5px;
  padding: 10px;
  margin-bottom: 10px;
}

.security-notes {
  border: 1px solid #c7c7c7;
  background: #f9f9f9;
  border-radius: 5px;
  padding: 10px;
  margin-top: 5px;
}

.table-fixed-head {
  overflow-y: auto;
  max-height: 500px;
}
.table-fixed-head thead th {
  position: sticky;
  top: 0;
}
.table-fixed-head table {
  border-collapse: collapse;
  width: 100%;
}
.table-fixed-head th,
.table-fixed-head td {
  padding: 8px 16px;
  font-size: 9pt;
}
.table-fixed-head th {
  background: #ededed;
  color: black;
  z-index: 1;
}
.table-fixed-head td {
  box-shadow: inset 1px 1px #999;
}
.table-fixed-head th {
  box-shadow: inset 1px 1px #999;
}
.table-fixed-head thead {
  box-shadow: inset 0px -1px #999;
}
.table-fixed-head table {
  border-right: solid 1px #999;
  border-bottom: solid 1px #999;
}

</style>
