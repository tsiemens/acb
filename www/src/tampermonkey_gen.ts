import { generateWsTamperScript, WsTaxEntry } from 'canada-acb';

export interface AcbTaxEntry {
   security: string;
   settlementDate: string; // YYYY-MM-DD
   affiliate: string;
   proceedsCad: number;
   costBaseCad: number;
   sellingExpensesCad: number;
}

function toWsEntry(entry: AcbTaxEntry): WsTaxEntry {
   return {
      description: `${entry.security} ${entry.settlementDate}`,
      settlementDate: entry.settlementDate,
      proceeds: entry.proceedsCad,
      costBase: entry.costBaseCad,
      expenses: entry.sellingExpensesCad,
   };
}

export function generateTamperMonkeyScript(
   entries: AcbTaxEntry[],
   affiliate: string,
   year: number,
): string {
   return generateWsTamperScript(
      entries.map(toWsEntry),
      year,
      { namespace: 'acb.ts0.ca/ws-tamper',
        label: `ACB [${affiliate}]`,
       }
   );
}
