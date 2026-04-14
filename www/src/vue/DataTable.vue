<template>
  <div>
    <div v-if="title" class="table-title">
      {{ title }}
      <span v-if="titleBadge" class="title-badge" :title="titleBadgeHelpText">{{ titleBadge }}</span>
    </div>

    <ErrorBox
      v-if="errors.length > 0"
      :store="{ visible: true, title: '', descPre: '', errorText: '', errorTexts: errors, descPost: errorSuffix }"
      :fullWidth="true"
    />

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
            <td
              v-for="(cell, ci) in row"
              :key="ci"
              :class="[cellClassFn ? cellClassFn(row, ci) : '', (cellTagClassFn && cellTagClassFn(row, ci)) ? 'cell-tag-td' : '']"
            >
              <span
                v-if="cellTagClassFn && cellTagClassFn(row, ci)"
                :class="['cell-tag', cellTagClassFn(row, ci)]"
              >{{ cell }}</span>
              <span
                v-else-if="cellHtmlFn && cellHtmlFn(cell, ci)"
                v-html="cellHtmlFn(cell, ci)"
              ></span>
              <span v-else v-html="formatCell(cell)"></span>
            </td>
          </tr>
          <tr v-if="table.footer">
            <td v-for="(cell, ci) in table.footer" :key="ci"><span v-html="formatCell(cell)"></span></td>
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
import ErrorBox from './ErrorBox.vue';

export default defineComponent({
   name: 'DataTable',
   components: { ErrorBox },
   props: {
      table: {
         type: Object as PropType<RenderTable>,
         required: true,
      },
      title: {
         type: String,
         default: '',
      },
      titleBadge: {
         type: String,
         default: '',
      },
      titleBadgeHelpText: {
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
      cellClassFn: {
         type: Function as PropType<(row: string[], colIndex: number) => string | string[] | null>,
         default: null,
      },
      cellTagClassFn: {
         type: Function as PropType<(row: string[], colIndex: number) => string | null>,
         default: null,
      },
      cellHtmlFn: {
         type: Function as PropType<(cell: string, colIndex: number) => string | null>,
         default: null,
      },
   },
   setup(props) {
      const errors = computed(() => props.table.errors || []);
      const notes = computed(() => props.table.notes || []);

      function escapeHtml(s: string): string {
         return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
      }

      function formatCell(cell: string): string {
         return escapeHtml(cell).replace(/\n/g, '<br>');
      }

      return { errors, notes, formatCell };
   },
});
</script>

<style scoped>
.table-title {
  padding-left: 8px;
  background: var(--primary-color-much-lighter);
  border-color: var(--primary-color-lighter);
  border-style: solid;
  border-width: thin;
  color: var(--dark-color);

  border-radius: 5px;
  margin-bottom: 5px;
  margin-top: 20px;
  font-weight: bold;
}

.title-badge {
  display: inline-block;
  margin-left: 8px;
  padding: 0px 8px;
  border-radius: 5px;
  font-size: 8pt;
  font-weight: 600;
  background: #d1fae5;
  color: #065f46;
  vertical-align: middle;
}

.title-badge[title]:not([title=""]) {
  cursor: help;
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
  padding: 4px 10px;
  font-size: 9pt;
}
.table-fixed-head th {
  background: #ededed;
  color: black;
  z-index: 1;
}
.table-fixed-head td {
  background: white;
  box-shadow: inset 1px 1px #d0d0d0;
}
.table-fixed-head th {
  box-shadow: inset 1px 1px #d0d0d0;
}
.table-fixed-head thead {
  box-shadow: inset 0px -1px #d0d0d0;
}
.table-fixed-head table {
  border-right: solid 1px #d0d0d0;
  border-bottom: solid 1px #d0d0d0;
}

.table-fixed-head tbody tr:hover td {
  filter: brightness(0.95);
}

.cell-tag-td {
  text-align: center;
}

.cell-tag {
  display: inline-block;
  padding: 1px 8px;
  border-radius: 5px;
  font-size: 8pt;
  font-weight: 500;
  min-width: 12ch;
  max-width: 12ch;
  text-align: center;
}

</style>
