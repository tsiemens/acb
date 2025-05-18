import { ElemBuilder } from "../dom_utils.js";
import { RunButton } from "./app_input.js";
import { ElementModel } from "./model_lib.js";

class FileDropAreaOuter extends ElementModel {
   public static readonly ID: string = "file-drop-area-outer";

   public static get(): FileDropAreaOuter {
      return new FileDropAreaOuter(
         ElementModel.getRequiredElementById(FileDropAreaOuter.ID));
   }
}

export class FileDropArea extends ElementModel {
   public static readonly ID: string = "file-drop-area";

   constructor(
      element: HTMLElement,
      private outerAreaElement: FileDropAreaOuter,
   ) {
      super(element);
   }

   public static get(): FileDropArea {
      return new FileDropArea(
         ElementModel.getRequiredElementById(FileDropArea.ID),
         FileDropAreaOuter.get()
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
         this.element.setAttribute("drop-active", "true");
         this.outerAreaElement.element.setAttribute("drop-active", "true");
      });

      this.element.addEventListener('dragleave', (_event) => {
         this.element.setAttribute("drop-active", "false");
         this.outerAreaElement.element.setAttribute("drop-active", "false");
      });

      this.element.addEventListener('drop', (event) => {
         event.stopPropagation();
         event.preventDefault();
         this.element.setAttribute("drop-active", "false");
         this.outerAreaElement.element.setAttribute("drop-active", "false");
         if (event.dataTransfer !== null) {
            const fileList: FileList = event.dataTransfer.files;
            addFilesToUse(fileList);
         }
      });
   }
}

export class FileSelectorInput extends ElementModel {
   public static readonly ID: string = "file-selector-input";

   public static get(): FileSelectorInput {
      return new FileSelectorInput(
         ElementModel.getRequiredElementById(FileSelectorInput.ID));
   }

   public setup(addFilesToUse: (arg0: FileList) => void) {
      this.element.addEventListener('input', (event) => {
         if (event.target) {
            const fileList = (event.target as HTMLInputElement).files;
            console.log("on input:", fileList);
            if (fileList) {
               addFilesToUse(fileList);
            }
         }
      });
   }
}

export class SelectedFileList extends ElementModel {
   public static readonly CLASS: string = "file-list";

   private static onRemoveFile: (fileId: number) => void;

   public static get(): SelectedFileList {
      return new SelectedFileList(
         ElementModel.getRequiredElementByQuery(`.${SelectedFileList.CLASS}`));
   }

   public setup(onRemoveFile: (fileId: number) => void) {
      SelectedFileList.onRemoveFile = onRemoveFile;
   }

   public addFileListEntry(fileId: number, fileName: string): void {
      const entry = new ElemBuilder('div')
         .classes(['file-list-item'])
         .build();

      const btn = new ElemBuilder('div')
         .classes(['button', 'b-skinny', 'b-light'])
         .text('X')
         .eventListener('click', (event) => {
            if (event.target) {
               const fileId = (event.target as HTMLElement).dataset.fileid;
               console.log("Click X button for fileId", fileId);

               if ( fileId !== undefined) {
                  SelectedFileList.onRemoveFile(parseInt(fileId));
                  SelectedFileList.get().removeFileListEntry(fileId);
               }
            }
         })
         .build();

      entry.appendChild(btn);

      const entryText = new ElemBuilder('div')
         .classes(['file-list-item-text'])
         .text(' ' + fileName)
         .build();
      entry.appendChild(entryText);

      entry.dataset.fileid = fileId.toString();

      this.element.appendChild(entry);

      if (this.element.children.length > 0) {
         RunButton.get().setEnabled(true);
      }
   }

   public removeFileListEntry(fileId: number | string): void {
      const fileIdStr = fileId.toString();

      const fileList = document.getElementsByClassName("file-list")[0];
      for (const child of fileList.children) {
         if ((child as HTMLElement).dataset.fileid == fileIdStr) {
            fileList.removeChild(child);
         }
      }
   
      if (fileList.children.length == 0) {
         RunButton.get().setEnabled(false);
      }
   }
}