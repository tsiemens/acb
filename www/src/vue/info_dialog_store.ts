import { reactive } from 'vue';

export interface InfoDialogStore {
   activeDialogId: string | null;
   dynamicTextTitle: string;
   dynamicTextContent: string;
}

let store: InfoDialogStore | null = null;

export function getInfoDialogStore(): InfoDialogStore {
   if (!store) {
      store = reactive({
         activeDialogId: null,
         dynamicTextTitle: '',
         dynamicTextContent: '',
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

export function openDynamicTextDialog(title: string, content: string): void {
   const s = getInfoDialogStore();
   s.dynamicTextTitle = title;
   s.dynamicTextContent = content;
   s.activeDialogId = 'dynamicTextInfo';
}

export function openTampermonkeyScriptDialog(title: string, content: string): void {
   const s = getInfoDialogStore();
   s.dynamicTextTitle = title;
   s.dynamicTextContent = content;
   s.activeDialogId = 'tampermonkeyScriptDialog';
}
