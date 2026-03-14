import { reactive } from 'vue';

export interface SidebarInfoStore {
   acbVersion: string;
   gitIssuesVisible: boolean;
   warningsSectionVisible: boolean;
}

let store: SidebarInfoStore | null = null;

export function getSidebarInfoStore(): SidebarInfoStore {
   if (!store) {
      store = reactive({
         acbVersion: 'v0.0.0',
         gitIssuesVisible: false,
         warningsSectionVisible: false,
      });
   }
   return store;
}
