import { reactive } from 'vue';

export interface ConfirmDialogStore {
   /** Whether the dialog is visible. */
   active: boolean;
   title: string;
   message: string;
   confirmLabel: string;
   cancelLabel: string;
   /** Resolve callback for the pending promise. */
   _resolve: ((confirmed: boolean) => void) | null;
}

let _store: ConfirmDialogStore | null = null;

export function getConfirmDialogStore(): ConfirmDialogStore {
   if (!_store) {
      _store = reactive({
         active: false,
         title: '',
         message: '',
         confirmLabel: 'OK',
         cancelLabel: 'Cancel',
         _resolve: null,
      });
   }
   return _store;
}

/**
 * Show a confirmation dialog and return a promise that resolves to
 * `true` (confirmed) or `false` (cancelled / dismissed).
 */
export function confirm(opts: {
   title: string;
   message: string;
   confirmLabel?: string;
   cancelLabel?: string;
}): Promise<boolean> {
   const store = getConfirmDialogStore();

   // If a dialog is already open, resolve it as cancelled.
   if (store._resolve) {
      store._resolve(false);
   }

   store.title = opts.title;
   store.message = opts.message;
   store.confirmLabel = opts.confirmLabel ?? 'OK';
   store.cancelLabel = opts.cancelLabel ?? 'Cancel';
   store.active = true;

   return new Promise<boolean>((resolve) => {
      store._resolve = resolve;
   });
}

export function resolveDialog(confirmed: boolean): void {
   const store = getConfirmDialogStore();
   const resolve = store._resolve;
   store.active = false;
   store._resolve = null;
   if (resolve) {
      resolve(confirmed);
   }
}
