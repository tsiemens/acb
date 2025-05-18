import { ElementModel } from "./model_lib.js";

export class InitSecErrors extends ElementModel {
   public static readonly ID: string = "init-secs-errors";

   public static get(): InitSecErrors {
      return new InitSecErrors(
         ElementModel.getRequiredElementById(InitSecErrors.ID));
   }

   public setErrors(errors: string[]) {
      this.element.innerText = errors.join("\n");
   }

   public clear() {
      this.element.innerText = "";
   }
}

export class ErrorBox extends ElementModel {
   public static readonly CLASS: string = "error-container";

   public static get(): ErrorBox {
      return new ErrorBox(
         ElementModel.getRequiredElementByQuery(`.${ErrorBox.CLASS}`));
   }

   // Override hide and show to use display instead of hidden attribute
   public hide() {
      this.element.style.display = 'none';
  }
  public show() {
      this.element.style.display = 'block';
  }

   public showWith(params: {
      title?: string, // - Title of the error box
      descPre?: string, // - First description part
      errorText?: string, // - Quoted/mono error text
      descPost?: string, // - Second description part following the error
   }) {
      const titleElement = ElementModel.getRequiredElementById('error-box-title');
      titleElement.textContent = params.title ? params.title : "Error";

      const descElement = ElementModel.getRequiredElementById('error-desc-pre');
      descElement.textContent = params.descPre ? params.descPre : "";

      const errorMessageElement = ElementModel.getRequiredElementById('error-message');
      errorMessageElement.textContent = params.errorText ? params.errorText : "";

      const postElement = ElementModel.getRequiredElementById('error-desc-post');
      postElement.textContent = params.descPost ? params.descPost : "";
      postElement.style.display = params.descPost ? 'block' : 'none';

      this.show();
   }
}