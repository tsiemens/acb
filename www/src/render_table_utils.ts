import { RenderTable } from "./acb_wasm_types.js";
import { affiliateBaseName } from "./vue/output_store.js";
import type { AcbTaxEntry } from "./tampermonkey_gen.js";

// -- Column indices for security render tables --
export const SETTLE_DATE_COL = 2;
export const ACTION_COL = 3;
export const AMOUNT_COL = 4;
export const AMT_PER_SHARE_COL = 6;
export const ACB_COL = 7;
export const COMMISSION_COL = 8;
export const CAP_GAIN_COL = 9;
export const AFFILIATE_COL = 14;
export const MEMO_COL = 15;

// A set of columns which have parentheses in them, which should be
// rendered on a separate line within the cell.
export const BREAK_BEFORE_PAREN_COLS = new Set([AMOUNT_COL, AMT_PER_SHARE_COL, COMMISSION_COL]);

export function isRegisteredAffiliate(affiliate: string): boolean {
   return /\(R\)\s*$/i.test(affiliate);
}

/**
 * Parse a rendered dollar string from the render table.
 * Handles formats like "$1,234.56", "-$1,234.56",
 * and multi-line FX strings like "$1,234.56\n(1000.00 USD)" where
 * the first line is the CAD value.
 * Returns 0 if the cell is empty or unparseable.
 */
export function parseDollarCell(cell: string): number {
   if (!cell) return 0;
   // Take only the first line (CAD value for FX strings)
   const firstLine = cell.split('\n')[0].trim();
   if (!firstLine || firstLine === '-' || firstLine === '--') return 0;
   // Strip SfL annotations: " *\n(SfL ...)" — already handled by first-line split,
   // but also strip trailing " *" or similar
   const cleaned = firstLine.replace(/\s*\*\s*$/, '');
   // Strip $ and commas, preserve leading -
   const numeric = cleaned.replace(/[$,]/g, '');
   const val = parseFloat(numeric);
   return isNaN(val) ? 0 : val;
}

export function collectSecuritiesWithErrors(
   securityTables: Map<string, RenderTable>
): string[] {
   const secsWithErrors: string[] = [];
   for (const [sec, table] of securityTables) {
      if (table.errors && table.errors.length > 0) {
         secsWithErrors.push(sec);
      }
   }
   secsWithErrors.sort();
   return secsWithErrors;
}

export interface SellDataFromTables {
   entries: AcbTaxEntry[];
   /** Sorted descending years that have sell transactions
     * (non-registered affiliates only). */
   sellYears: number[];
   /** Base affiliate names that have sell transactions. */
   affiliateBaseNames: string[];
}

/**
 * Scan all security render tables and extract sell transaction data
 * for non-registered affiliates.
 */
export function extractSellData(
   securityTables: Map<string, RenderTable>,
): SellDataFromTables {
   const entries: AcbTaxEntry[] = [];
   const yearsSet = new Set<number>();
   const affiliatesSet = new Set<string>();

   for (const [security, table] of securityTables) {
      for (const row of table.rows) {
         const action = (row[ACTION_COL] || '').trim();
         if (action.toLowerCase() !== 'sell') continue;

         const affiliate = row[AFFILIATE_COL] || '';
         if (isRegisteredAffiliate(affiliate)) continue;

         const baseName = affiliateBaseName(affiliate);
         affiliatesSet.add(baseName);

         const settlDate = row[SETTLE_DATE_COL] || '';
         const yearStr = settlDate.split('-')[0];
         const year = parseInt(yearStr, 10);
         if (!isNaN(year)) yearsSet.add(year);

         entries.push({
            security,
            settlementDate: settlDate,
            affiliate: baseName,
            proceedsCad: parseDollarCell(row[AMOUNT_COL]),
            costBaseCad: parseDollarCell(row[ACB_COL]),
            sellingExpensesCad: parseDollarCell(row[COMMISSION_COL]),
         });
      }
   }

   const sellYears = Array.from(yearsSet).sort((a, b) => b - a);
   const affiliateBaseNames = Array.from(affiliatesSet).sort();
   return { entries, sellYears, affiliateBaseNames };
}

export function generateShareTallyRenderTable(txSummary: RenderTable): [RenderTable, string] {
   const secIdx = txSummary.header.indexOf("security");
   if (secIdx < 0) {
      throw new Error(`Expected 'security' column in header, found: ${txSummary.header.join(', ')}`);
   }
   const sharesIdx = txSummary.header.indexOf("shares");
   if (sharesIdx < 0) {
      throw new Error(`Expected 'shares' column in header, found: ${txSummary.header.join(', ')}`);
   }
   const affIdx = txSummary.header.indexOf("affiliate");
   const hasMultipleAffiliates = affIdx >= 0 &&
      new Set(txSummary.rows.map(r => r[affIdx])).size > 1;

   const header = hasMultipleAffiliates
      ? ["security", "affiliate", "shares"]
      : ["security", "shares"];
   let csvText = header.join(",") + "\n";
   const rows = txSummary.rows.map((row) => {
      const tallyRow = hasMultipleAffiliates
         ? [row[secIdx], row[affIdx], row[sharesIdx]]
         : [row[secIdx], row[sharesIdx]];
      csvText += tallyRow.join(",") + "\n";
      return tallyRow;
   });
   const table = new RenderTable(
      header, rows, txSummary.footer, txSummary.notes, txSummary.errors);
   return [table, csvText];
}
