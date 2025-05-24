import { ElementModel } from "./model_lib.js";

export class InfoDialogBackdrop extends ElementModel {
   public static readonly ID: string = "dialogBackdrop";

   public static get(): InfoDialogBackdrop {
      return new InfoDialogBackdrop(
         ElementModel.getRequiredElementById(InfoDialogBackdrop.ID));
   }

   /** @override */
   public setHidden(hidden: boolean): void {
      this.element.style.display = (hidden ? 'none' : 'block');
  }
  /** @override */
  public isHidden(): boolean {
      return this.element.style.display === 'none';
  }
}

export class InfoDialog extends ElementModel {
   public static readonly CLASS: string = "info-dialog";
   public static readonly CLOSE_BTN_CLASS: string = "info-dialog-close";

   public static initAll() {
      // Setup close buttons
      const closeButtons = document.querySelectorAll(`.${InfoDialog.CLOSE_BTN_CLASS}`);
      closeButtons.forEach((button) => {
         button.addEventListener("click", (_event) => {
            InfoDialog.closeAll();
         });
      }
      );
   }

   public static closeAll() {
      const infoDialogs = document.querySelectorAll(`.${InfoDialog.CLASS}`);
      infoDialogs.forEach((dialog) => {
         dialog.classList.remove('active');
      });
      InfoDialogBackdrop.get().hide();
   }

   public static getById(id: string): InfoDialog {
      const dialog = ElementModel.getRequiredElementById(id);
      return new InfoDialog(dialog);
   }

   /** @override */
   public setHidden(hidden: boolean): void {
      if (hidden) {
         this.element.classList.remove('active');
      } else {
         this.element.classList.add('active');
      }
  }
  /** @override */
  public isHidden(): boolean {
      return !this.element.classList.contains('active');
  }

}

// Sidebar items to open info dialogs
export class InfoListItem extends ElementModel {
   static readonly CLASS: string = "clickable-info-item";

   public static initAll() {
      // Setup click handlers
      const infoListItems = document.querySelectorAll(`.${InfoListItem.CLASS}`);
      infoListItems.forEach((item) => {
         item.addEventListener("click", (event) => {
            if (!event.target) {
               return;
            }
            const element = (event.target as Element).closest(`.${InfoListItem.CLASS}`);
            if (!element) {
               console.error("No info list item found", event.target);
               return;
            }
            const dialogId = element.getAttribute("data-dialog-id");
            if (!dialogId) {
               // Try get from parent
               console.error("No dialog ID found for info list item", element);
               return;
            }
            const infoDialog = InfoDialog.getById(dialogId);
            infoDialog.show();
            InfoDialogBackdrop.get().show();
         });
      });
   }
}
