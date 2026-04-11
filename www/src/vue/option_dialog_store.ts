import { reactive } from 'vue';

export interface DialogOption {
   /** Stable identifier returned when this option is chosen. */
   id: string;
   /** User-visible button label. */
   text: string;
   /**
    * If true, the button is styled with the primary/affirmative color.
    * Otherwise it uses the neutral cancel color.
    */
   affirmative: boolean;
}

export interface OptionDialogStore {
   /** Whether the dialog is visible. */
   active: boolean;
   title: string;
   message: string;
   options: DialogOption[];
   /** Resolve callback for the pending promise. */
   _resolve: ((id: string | null) => void) | null;
}

let _store: OptionDialogStore | null = null;

export function getOptionDialogStore(): OptionDialogStore {
   if (!_store) {
      _store = reactive({
         active: false,
         title: '',
         message: '',
         options: [],
         _resolve: null,
      });
   }
   return _store;
}

/**
 * Show a dialog with arbitrary options and return a promise that resolves
 * to the `id` of the chosen option, or `null` if the dialog was dismissed
 * (backdrop click, Escape, or close button).
 */
export function showOptionDialog(opts: {
   title: string;
   message: string;
   options: DialogOption[];
}): Promise<string | null> {
   const store = getOptionDialogStore();

   // If a dialog is already open, resolve it as dismissed.
   if (store._resolve) {
      store._resolve(null);
   }

   store.title = opts.title;
   store.message = opts.message;
   store.options = opts.options;
   store.active = true;

   return new Promise<string | null>((resolve) => {
      store._resolve = resolve;
   });
}

export function resolveOptionDialog(id: string | null): void {
   const store = getOptionDialogStore();
   const resolve = store._resolve;
   store.active = false;
   store._resolve = null;
   if (resolve) {
      resolve(id);
   }
}
