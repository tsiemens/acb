import { setTextContentToTextWithNewlines } from "../dom_utils.js";
import { ElementModel } from "./model_lib.js";

export class ErrorBox extends ElementModel {
   public static readonly CLASS: string = "error-container";

   public static readonly MAIN_ERRORS_ID: string = "mainErrorContainer";
   public static readonly GIT_ERRORS_ID: string = "gitIssuesErrorContainer";

   public static get(id: string): ErrorBox {
      return new ErrorBox(
         ElementModel.getRequiredElementById(id));
   }

   public static getMain(): ErrorBox {
      return ErrorBox.get(ErrorBox.MAIN_ERRORS_ID);
   }

   public static getGitIssues(): ErrorBox {
      return ErrorBox.get(ErrorBox.GIT_ERRORS_ID);
   }

   /** @override */
   public setHidden(hidden: boolean): void {
      this.element.style.display = (hidden ? 'none' : 'block');
  }
  /** @override */
  public isHidden(): boolean {
      return this.element.style.display === 'none';
  }

   public showWith(params: {
      title?: string, // - Title of the error box
      descPre?: string, // - First description part
      errorText?: string, // - Quoted/mono error text
      descPost?: string, // - Second description part following the error
   }) {
      const titleElement = this.getRequiredSubElementByClass('error-box-title');
      titleElement.textContent = params.title ? params.title : "Error";

      const descElement = this.getRequiredSubElementByClass('error-desc-pre');
      setTextContentToTextWithNewlines(descElement, params.descPre ? params.descPre : "");
      descElement.style.display = params.descPre ? 'block' : 'none';

      const errorMessageElement = this.getRequiredSubElementByClass('error-message');
      setTextContentToTextWithNewlines(errorMessageElement, params.errorText ? params.errorText : "");
      errorMessageElement.style.display = params.errorText ? 'block' : 'none';

      const postElement = this.getRequiredSubElementByClass('error-desc-post');
      setTextContentToTextWithNewlines(postElement, params.descPost ? params.descPost : "");
      postElement.style.display = params.descPost ? 'block' : 'none';

      this.show();
   }
}

export class SidebarWarningsSection extends ElementModel {
   public static readonly CLASS: string = "info-warnings-section";

   public static get(): SidebarWarningsSection {
      return new SidebarWarningsSection(
         ElementModel.getRequiredElementByQuery(`.${SidebarWarningsSection.CLASS}`));
   }
}