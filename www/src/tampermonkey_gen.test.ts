import { describe, it, expect } from 'vitest';
import {
   AcbTaxEntry,
   buildEntryDescription,
   compactDate,
   stripTrailingDate,
} from './tampermonkey_gen.js';
import { affiliateCostPoolTag } from './vue/output_store.js';

function entry(partial: Partial<AcbTaxEntry>): AcbTaxEntry {
   return {
      security: 'ANET',
      settlementDate: '2026-02-20',
      affiliate: 'Default',
      proceedsCad: 0,
      costBaseCad: 0,
      sellingExpensesCad: 0,
      ...partial,
   };
}

describe('stripTrailingDate', () => {
   it('removes a trailing ISO date', () => {
      expect(stripTrailingDate('7(1.31) - RSU 123456 2017-02-01'))
         .toBe('7(1.31) - RSU 123456');
   });

   it('leaves a string without a trailing date unchanged (but trimmed)', () => {
      expect(stripTrailingDate('  7(1.31) - RSU 123456  ')).toBe('7(1.31) - RSU 123456');
   });

   it('only strips a date at the very end', () => {
      expect(stripTrailingDate('2017-02-01 RSU 123456')).toBe('2017-02-01 RSU 123456');
   });
});

describe('compactDate', () => {
   it('compacts a YYYY-MM-DD date to YYMMDD', () => {
      expect(compactDate('2026-02-20')).toBe('260220');
      expect(compactDate('1999-12-31')).toBe('991231');
   });

   it('returns malformed input unchanged', () => {
      expect(compactDate('2026-2-20')).toBe('2026-2-20');
      expect(compactDate('')).toBe('');
   });
});

describe('affiliateCostPoolTag', () => {
   it('extracts the bracketed tag contents', () => {
      expect(affiliateCostPoolTag('Default [7(1.31) - RSU 123456 2017-02-01]'))
         .toBe('7(1.31) - RSU 123456 2017-02-01');
   });

   it('works alongside a registered marker', () => {
      expect(affiliateCostPoolTag('Spouse (R) [7(1.31) - ESO A 2020-01-01]'))
         .toBe('7(1.31) - ESO A 2020-01-01');
   });

   it('returns null when there is no marker', () => {
      expect(affiliateCostPoolTag('Default')).toBeNull();
      expect(affiliateCostPoolTag('Default (R)')).toBeNull();
   });

   it('returns null for an empty marker', () => {
      expect(affiliateCostPoolTag('Default []')).toBeNull();
      expect(affiliateCostPoolTag('Default [   ]')).toBeNull();
   });
});

describe('buildEntryDescription', () => {
   it('renders ordinary (non-pool) entries with the full settlement date', () => {
      expect(buildEntryDescription(entry({}))).toBe('ANET 2026-02-20');
   });

   it('folds a cost-pool tag in and fits exactly within 30 chars', () => {
      const desc = buildEntryDescription(entry({
         costPoolTag: '7(1.31) - RSU 123456 2017-02-01',
      }));
      expect(desc).toBe('ANET 7(1.31) RSU 123456 260220');
      expect(desc.length).toBe(30);
   });

   it('keeps the trailing chars of the detail when truncating', () => {
      // Longer symbol leaves less room: the leading "R" of "RSU" is dropped so
      // the full award number survives.
      const desc = buildEntryDescription(entry({
         security: 'GOOGL',
         costPoolTag: '7(1.31) - RSU 123456 2017-02-01',
      }));
      expect(desc).toBe('GOOGL 7(1.31) SU 123456 260220');
      expect(desc.length).toBeLessThanOrEqual(30);
   });

   it('keeps short details intact', () => {
      const desc = buildEntryDescription(entry({
         costPoolTag: '7(1.31) - ESO 2017-02-01',
      }));
      expect(desc).toBe('ANET 7(1.31) ESO 260220');
   });

   it('drops the detail entirely when there is no room', () => {
      const desc = buildEntryDescription(entry({
         security: 'VERYLONGSYMBOL',
         costPoolTag: '7(1.31) - RSU 123456 2017-02-01',
      }));
      expect(desc.length).toBeLessThanOrEqual(30);
      expect(desc.startsWith('VERYLONGSYMBOL 7(1.31)')).toBe(true);
   });

   it('handles a tag with no grant detail', () => {
      const desc = buildEntryDescription(entry({
         costPoolTag: '7(1.31) 2017-02-01',
      }));
      expect(desc).toBe('ANET 7(1.31) 260220');
   });

   it('never exceeds the WS 30-char limit', () => {
      const cases: AcbTaxEntry[] = [
         entry({ costPoolTag: '7(1.31) - RSU 123456 2017-02-01' }),
         entry({ security: 'GOOGL', costPoolTag: '7(1.31) - Option Grant 99887766 2020-11-30' }),
         entry({ security: 'BRK.B', costPoolTag: '7(1.31) - ESO B 2019-06-15' }),
      ];
      for (const c of cases) {
         expect(buildEntryDescription(c).length).toBeLessThanOrEqual(30);
      }
   });
});
