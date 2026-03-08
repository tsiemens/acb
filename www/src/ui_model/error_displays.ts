import { getErrorBoxStore } from '../vue/error_box_store.js';
import { ElementModel } from './model_lib.js';

export class ErrorBox {
   public static readonly MAIN_ERRORS_ID: string = "mainErrorContainer";
   public static readonly GIT_ERRORS_ID: string = "gitIssuesErrorContainer";

   private constructor(private id: string) {}

   public static getMain(): ErrorBox {
      return new ErrorBox(ErrorBox.MAIN_ERRORS_ID);
   }

   public static getGitIssues(): ErrorBox {
      return new ErrorBox(ErrorBox.GIT_ERRORS_ID);
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

export class SidebarWarningsSection extends ElementModel {
   public static readonly CLASS: string = "info-warnings-section";

   public static get(): SidebarWarningsSection {
      return new SidebarWarningsSection(
         ElementModel.getRequiredElementByQuery(`.${SidebarWarningsSection.CLASS}`));
   }
}
