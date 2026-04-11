import { showOptionDialog } from './option_dialog_store.js';

const CONFIRM_ID = 'confirm';
const CANCEL_ID = 'cancel';

/**
 * Show a confirmation dialog and return a promise that resolves to
 * `true` (confirmed) or `false` (cancelled / dismissed).
 *
 * This is a thin wrapper around the generic option dialog, preserving
 * the simple two-button confirm/cancel interface used throughout the app.
 */
export async function confirm(opts: {
   title: string;
   message: string;
   confirmLabel?: string;
   cancelLabel?: string;
}): Promise<boolean> {
   const id = await showOptionDialog({
      title: opts.title,
      message: opts.message,
      options: [
         {
            id: CANCEL_ID,
            text: opts.cancelLabel ?? 'Cancel',
            affirmative: false,
         },
         {
            id: CONFIRM_ID,
            text: opts.confirmLabel ?? 'OK',
            affirmative: true,
         },
      ],
   });
   return id === CONFIRM_ID;
}
