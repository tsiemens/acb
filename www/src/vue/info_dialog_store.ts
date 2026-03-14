import { reactive } from 'vue';

export interface InfoDialogStore {
   activeDialogId: string | null;
}

let store: InfoDialogStore | null = null;

export function getInfoDialogStore(): InfoDialogStore {
   if (!store) {
      store = reactive({
         activeDialogId: null,
      });
   }
   return store;
}

export function openDialog(dialogId: string): void {
   getInfoDialogStore().activeDialogId = dialogId;
}

export function closeDialog(): void {
   getInfoDialogStore().activeDialogId = null;
}
