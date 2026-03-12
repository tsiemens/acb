import { ElementModel } from "./model_lib.js";

export class FileDropArea extends ElementModel {
   public static readonly ID: string = "fileDropArea";

   public static get(): FileDropArea {
      return new FileDropArea(
         ElementModel.getRequiredElementById(FileDropArea.ID),
      );
   }

   public setup(addFilesToUse: (arg0: FileList) => void) {
      this.element.addEventListener('dragover', (event) => {
         event.stopPropagation();
         event.preventDefault();
         // Style the drag-and-drop as a "copy file" operation.
         if (event.dataTransfer !== null) {
            event.dataTransfer.dropEffect = 'copy';
         }
         // This is required because :hover is not set if we're dragging from
         // another window and the browser is not in focus.
         this.element.setAttribute("drop-active", "true");
      });

      this.element.addEventListener('dragleave', (_event) => {
         this.element.setAttribute("drop-active", "false");
      });

      this.element.addEventListener('drop', (event) => {
         event.stopPropagation();
         event.preventDefault();
         this.element.setAttribute("drop-active", "false");
         if (event.dataTransfer !== null) {
            const fileList: FileList = event.dataTransfer.files;
            addFilesToUse(fileList);
         }
      });
   }
}

export class FileSelectorInput extends ElementModel {
   public static readonly ID: string = "fileSelectorInput";

   public static get(): FileSelectorInput {
      return new FileSelectorInput(
         ElementModel.getRequiredElementById(FileSelectorInput.ID));
   }

   public setup(addFilesToUse: (arg0: FileList) => void) {
      this.element.addEventListener('input', (event) => {
         console.log("FileSelectorInput input event");
         if (event.target) {
            const fileList = (event.target as HTMLInputElement).files;
            console.log("on input:", fileList);
            if (fileList) {
               addFilesToUse(fileList);
            }
            // Reset tthe input files so that it can be re-selected
            // and we still get a change event.
            (event.target as HTMLInputElement).files = null;
            (event.target as HTMLInputElement).value = '';
         }
      });
   }
}
