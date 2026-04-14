import { reactive } from 'vue';

export interface ErrorBoxState {
   visible: boolean;
   title: string;
   descPre: string;
   errorText: string;
   errorTexts?: string[];
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
   let store = stores.get(id);
   if (!store) {
      store = makeState();
      stores.set(id, store);
   }
   return store;
}

export class ErrorBox {
   public static readonly MAIN_ERRORS_ID: string = "mainErrorContainer";
   public static readonly BROKER_CONVERT_ERRORS_ID: string = "brokerConvertErrorContainer";
   public static readonly BROKER_CONVERT_WARNINGS_ID: string = "brokerConvertWarningContainer";
   public static readonly GIT_ERRORS_ID: string = "gitIssuesErrorContainer";
   public static readonly VERSION_WARNING_ID: string = "versionWarningContainer";

   private constructor(private id: string) {}

   public static getMain(): ErrorBox {
      return new ErrorBox(ErrorBox.MAIN_ERRORS_ID);
   }

   public static getBrokerConvert(): ErrorBox {
      return new ErrorBox(ErrorBox.BROKER_CONVERT_ERRORS_ID);
   }

   public static getBrokerConvertWarnings(): ErrorBox {
      return new ErrorBox(ErrorBox.BROKER_CONVERT_WARNINGS_ID);
   }

   public static getGitIssues(): ErrorBox {
      return new ErrorBox(ErrorBox.GIT_ERRORS_ID);
   }

   public static getVersionWarning(): ErrorBox {
      return new ErrorBox(ErrorBox.VERSION_WARNING_ID);
   }

   public hide(): void {
      getErrorBoxStore(this.id).visible = false;
   }

   public show(): void {
      getErrorBoxStore(this.id).visible = true;
   }

   public showWith(params: {
      title?: string;
      descPre?: string;
      errorText?: string;
      descPost?: string;
   }) {
      const store = getErrorBoxStore(this.id);
      store.title = params.title ?? 'Error';
      store.descPre = params.descPre ?? '';
      store.errorText = params.errorText ?? '';
      store.descPost = params.descPost ?? '';
      store.visible = true;
   }
}
