import { generateWsTamperScript, WsTaxEntry } from 'canada-acb';

export interface AcbTaxEntry {
   security: string;
   settlementDate: string; // YYYY-MM-DD
   affiliate: string;
   /** The affiliate's cost pool tag, if it is an isolated ACB cost pool (e.g.
     * "7(1.31) - RSU 123456 2017-02-01"). Folded into the WS description. */
   costPoolTag?: string;
   proceedsCad: number;
   costBaseCad: number;
   sellingExpensesCad: number;
}

// Wealthsimple truncates the disposition Description field at 30 chars, and the
// WS form has no separate date field — the settlement date lives only inside
// this text. So descriptions must be kept within this budget.
const WS_DESC_MAX_LEN = 30;

/** Strip a trailing ISO date (YYYY-MM-DD) from a string, if one is present.
  * Returns the (trimmed) input unchanged when there is no trailing date. */
export function stripTrailingDate(s: string): string {
   return s.replace(/\s*\d{4}-\d{2}-\d{2}\s*$/, '').trim();
}

/** Compact a YYYY-MM-DD date to YYMMDD (e.g. "2026-02-20" -> "260220"). Returns
  * the input unchanged if it isn't a well-formed ISO date. */
export function compactDate(isoDate: string): string {
   const m = /^\d{2}(\d{2})-(\d{2})-(\d{2})$/.exec(isoDate);
   return m ? `${m[1]}${m[2]}${m[3]}` : isoDate;
}

/**
 * Build the WS disposition description for an entry.
 *
 * Ordinary entries render as "{security} {settlementDate}" (unchanged).
 *
 * Cost-pool (e.g. ITA 7(1.31)) entries fold in their tag. The tag looks like
 * "7(1.31) - RSU 123456 2017-02-01": a leading flag, a " - "-separated grant
 * detail, and a trailing acquisition date. We drop the trailing acquisition
 * date (the sale date is what belongs here), keep the flag verbatim, and append
 * the compacted sale date, giving the grant detail whatever room is left within
 * the 30-char limit. The detail is truncated from the front so its most
 * specific part (e.g. the award number) survives:
 *   "ANET 7(1.31) RSU 123456 260220"
 */
export function buildEntryDescription(entry: AcbTaxEntry): string {
   const { security, settlementDate, costPoolTag } = entry;
   if (!costPoolTag) {
      return `${security} ${settlementDate}`;
   }

   const tag = stripTrailingDate(costPoolTag);
   const sepIdx = tag.indexOf(' - ');
   const flag = (sepIdx >= 0 ? tag.slice(0, sepIdx) : tag).trim();
   let detail = (sepIdx >= 0 ? tag.slice(sepIdx + 3) : '').trim();

   const compactedDate = compactDate(settlementDate);
   const prefix = `${security} ${flag} `;
   const suffix = ` ${compactedDate}`;
   const detailBudget = WS_DESC_MAX_LEN - prefix.length - suffix.length;

   let desc: string;
   if (detail.length > 0 && detailBudget > 0) {
      if (detail.length > detailBudget) {
         // Keep the trailing chars (the unique identifier) over the leading ones.
         detail = detail.slice(detail.length - detailBudget);
      }
      desc = `${prefix}${detail}${suffix}`;
   } else {
      // No room for (or no) grant detail: just the flag and compacted date.
      desc = `${security} ${flag}${suffix}`;
   }
   // Final guard: a very long symbol can still overrun the limit.
   return desc.slice(0, WS_DESC_MAX_LEN);
}

function toWsEntry(entry: AcbTaxEntry): WsTaxEntry {
   return {
      description: buildEntryDescription(entry),
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
      { namespace: 'acb.trevors.dev/ws-tamper',
        label: `ACB [${affiliate}]`,
       }
   );
}
