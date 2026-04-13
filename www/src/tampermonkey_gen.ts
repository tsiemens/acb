export interface AcbTaxEntry {
   security: string;
   date: string; // YYYY-MM-DD
   affiliate: string;
   proceedsCad: number;
   costBaseCad: number;
   sellingExpensesCad: number;
}

interface WsTaxEntry {
   typeKey: string; // defaults to 'p'
   description: string;
   proceeds: number;
   costBase: number;
   expenses: number;
   period: number; // defaults to 0
}

function toWsEntry(entry: AcbTaxEntry): WsTaxEntry {
   return {
      typeKey: "p",
      description: `${entry.security} ${entry.date}`,
      proceeds: entry.proceedsCad,
      costBase: entry.costBaseCad,
      expenses: entry.sellingExpensesCad,
      period: 0,
   };
}

export function generateTamperMonkeyScript(entries: AcbTaxEntry[]): string {
   const wsEntries = entries.map(toWsEntry);
   const entriesJson = JSON.stringify(wsEntries, null, 2)
      .split('\n')
      .map((line, i) => (i === 0 ? line : '   ' + line))
      .join('\n');

   return `// ==UserScript==
// @name         ACB Capital Gains — WealthSimple Tax Auto-Fill
// @namespace    https://github.com/tsiemens/acb
// @version      0.1.0
// @description  Injects ACB capital gains entries into WealthSimple Tax
// @author       ACB Tool
// @match        https://my.wealthsimple.com/*
// @grant        none
// @run-at       document-idle
// ==/UserScript==

(function () {
   'use strict';

   const ACB_ENTRIES = ${entriesJson};

   console.log('[ACB] WealthSimple Tax auto-fill script loaded — ' + ACB_ENTRIES.length + ' entries ready.');
   console.log('[ACB] Entries:', ACB_ENTRIES);
})();
`;
}
