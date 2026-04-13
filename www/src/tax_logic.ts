// Canadian personal tax deadline is April 30.
// Before the deadline, the "current" tax year is last year.
// After the deadline, the "current" tax year is this year.
const TAX_DEADLINE_MONTH = 4; // April (1-indexed)
const TAX_DEADLINE_DAY = 30;

export function getCurrentTaxYear(): number {
   const now = new Date();
   const year = now.getFullYear();
   const month = now.getMonth() + 1;
   const day = now.getDate();
   if (month < TAX_DEADLINE_MONTH || (month === TAX_DEADLINE_MONTH && day <= TAX_DEADLINE_DAY)) {
      return year - 1;
   }
   return year;
}
