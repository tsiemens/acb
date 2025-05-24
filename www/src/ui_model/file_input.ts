import { ElemBuilder } from "../dom_utils.js";
import { RunButton } from "./app_input.js";
import { ButtonElementModel, ElementModel } from "./model_lib.js";

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
         .classes(['file-item'])
         .build();

      const entryText = new ElemBuilder('span')
         .classes(['file-item-name'])
         .text(fileName)
         .build();
      entry.appendChild(entryText);

      const btn = new ElemBuilder('button')
         .classes(['file-remove-btn'])
         .innerHTML('&times;')
         .eventListener('click', (event) => {
            if (event.target) {
               const entry = (event.target as HTMLElement).closest('.file-item');
               if (!entry) {
                  console.error("Could not find file-item for event target");
                  return;
               }
               const fileId = (entry as HTMLElement).dataset.fileid;
               console.log("Click X button for fileId", fileId);

               if (fileId !== undefined) {
                  SelectedFileList.onRemoveFile(parseInt(fileId));
                  SelectedFileList.get().removeFileListEntry(fileId);
               }
            }
         })
         .build();

      entry.appendChild(btn);

      entry.dataset.fileid = fileId.toString();

      this.element.appendChild(entry);

      if (this.element.children.length > 0) {
         RunButton.get().setEnabled(true);
         ClearFilesButton.get().setEnabled(true);
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
         ClearFilesButton.get().setEnabled(false);
      }
   }

   public removeAllFiles(): void {
      console.log("SelectedFileList::removeAllFiles");

      const listItems = this.element.getElementsByClassName("file-item");
      const fileIds = [];
      for (const listItem of listItems) {
         const fileId = (listItem as HTMLElement).dataset.fileid;
         if (fileId !== undefined) {
            fileIds.push(fileId);
         }
      }
      for (const fileId of fileIds) {
         SelectedFileList.onRemoveFile(parseInt(fileId));
         SelectedFileList.get().removeFileListEntry(fileId);
      }
   }
}

export class ClearFilesButton extends ButtonElementModel {
   public static readonly ID: string = "clearFilesButton";

   public static get(): ClearFilesButton {
      return new ClearFilesButton(
         ElementModel.getRequiredElementById(ClearFilesButton.ID));
   }

   public setup() {
      this.setClickListener((_event) => {
         SelectedFileList.get().removeAllFiles();
      });
   }
}