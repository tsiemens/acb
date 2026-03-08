import { reactive } from 'vue';

export interface ErrorBoxState {
   visible: boolean;
   title: string;
   descPre: string;
   errorText: string;
   descPost: string;
}

function makeState(): ErrorBoxState {
   return reactive({
      visible: false,
      title: 'Error',
      descPre: '',
      errorText: '',
      descPost: '',
   });
}

const stores = new Map<string, ErrorBoxState>();

export function getErrorBoxStore(id: string): ErrorBoxState {
   if (!stores.has(id)) {
      stores.set(id, makeState());
   }
   return stores.get(id)!;
}
