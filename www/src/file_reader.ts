class FileLoadError {
   constructor(
      public fileName: string,
      public errorDesc: string,
      // Extra error info coming from lower down
      public error: string,
   ) {}
}

export class FileLoadQueue {
   public filesToLoad: File[];
   public loadedContent: string[];
   public remainingToLoad: number;
   public loadErrors: FileLoadError[];

   constructor(filesToLoad: File[]) {
      this.filesToLoad = filesToLoad;
      // set loadedContent and loadedFileNames to arrays the same size as
      // pendingFiles, filled with empty strings
      this.loadedContent = new Array<string>(filesToLoad.length).fill("");
      this.remainingToLoad = filesToLoad.length;
      this.loadErrors = [];
   }
}

export class FileLoadResult {
   constructor(
      public loadedFileNames: string[],
      public loadedContent: string[],
      public loadErrors: FileLoadError[],
) {}

   public static fromFileLoadQueue(
      loadQueue: FileLoadQueue,
   ): FileLoadResult {
      return new FileLoadResult(
         loadQueue.filesToLoad.map((file) => file.name),
         loadQueue.loadedContent,
         loadQueue.loadErrors,
      );
   }
}

export function printMetadataForFileList(fileList: FileList) {
   for (const file of fileList) {
      // Not supported in Safari for iOS.
      const name = file.name ? file.name : 'NOT SUPPORTED';
      // Not supported in Firefox for Android or Opera for Android.
      const type = file.type ? file.type : 'NOT SUPPORTED';
      // Unknown cross-browser support.
      const size = file.size ? file.size : 'NOT SUPPORTED';
      let data = {file, name, type, size};
      console.log(data);
   }
}

class FilesLoader {
   private loadQueue: FileLoadQueue;
   constructor(filesToLoad: File[]) {
      this.loadQueue = new FileLoadQueue(filesToLoad);
   }

   protected fileSupported(_file: File): boolean {
      return true;
   }

   private readFile(
         fileIndex: number,
         onComplete: (_: FileLoadResult) => void,
      ) {
      const file = this.loadQueue.filesToLoad[fileIndex];
      if (!this.fileSupported(file)) {
         this.loadQueue.remainingToLoad--;
         if (this.loadQueue.remainingToLoad == 0) {
            onComplete(
               FileLoadResult.fromFileLoadQueue(this.loadQueue));
         }
         return;
      }

     const reader = new FileReader();
     reader.addEventListener('loadend', (event) => {
         this.loadQueue.remainingToLoad--;

         const result: string | ArrayBuffer | null =
            event.target ? event.target.result : null;
         console.log('FileReader loaded:', result);

         if (!result) {
            this.loadQueue.loadErrors.push(
               new FileLoadError(
                  file.name,
                  "Error reading " + file.name,
                  event.target && event.target.error ?
                     `${event.target.error.name}:${event.target.error.message}` :
                     "Unknown error",
               ));
            return;
         } else {
            // If result is an ArrayBuffer, convert it to a string.
            let resultStr: string;
            if (result instanceof ArrayBuffer) {
               const decoder = new TextDecoder("utf-8");
               const content = decoder.decode(result);
               resultStr = content;
            } else {
               resultStr = result;
            }

            // Decode base64
            const b64Content = resultStr.split(";base64,")[1];
            const content = atob(b64Content);

            this.loadQueue.loadedContent[fileIndex] = content;
         }

         console.debug("FileLoader.readFile: remaining to load:",
                       this.loadQueue.remainingToLoad);
         if (this.loadQueue.remainingToLoad == 0) {
            onComplete(
               FileLoadResult.fromFileLoadQueue(this.loadQueue));
         }
     });
     reader.readAsDataURL(file);
   }

   public loadFiles(
         onComplete: (_: FileLoadResult) => void,
      ) {
      for (let i = 0; i < this.loadQueue.filesToLoad.length; i++) {
         this.readFile(i, onComplete);
      }
   }
}

export class CsvFilesLoader extends FilesLoader {
   protected fileSupported(file: File): boolean {
      // Check if the file is a CSV and not an image or something.
      if (file.type && file.type.indexOf('text/csv') === -1) {
         console.log('File is not a csv.', file.type, file);
         return false;
      }
      return true;
   }
}

// Takes a list of file names
export class FileStager {
   private filesToUse: Map<number, File>;
   private nextFileIdx: number;

   public static globalInstance: FileStager = new FileStager();

   constructor() {
      this.filesToUse = new Map<number, File>();
      this.nextFileIdx = 1;
   }

   // To be called by the drop area's drop event handler.
   public addFilesToUse(fileList: FileList): void {
      for (const file of fileList) {
         this.addFileToUse(file);
      }
   }

   public addFileToUse(file: File): number {
      const fileIdx = this.nextFileIdx;
      this.filesToUse.set(fileIdx, file);
      this.nextFileIdx++;
      return fileIdx
   }

   public removeFile(fileId: number): void {
      this.filesToUse.delete(fileId);
   }

   public getFilesToUseList(): File[] {
      const fileList: File[] = [];
      for (const fileId of this.filesToUse.keys()) {
         const file = this.filesToUse.get(fileId);
         if (file === undefined) {
            continue;
         }
         fileList.push(file);
      }
      return fileList;
   }

   public isFileSelected(file: File): boolean {
      for (const fileId of this.filesToUse.keys()) {
         const selFile = this.filesToUse.get(fileId);
         if (selFile === undefined) {
            continue;
         }
         if (selFile.name == file.name && selFile.lastModified == file.lastModified) {
            return true;
         }
      }
      return false;
   }

   public loadFiles(onComplete: (_: FileLoadResult) => void): void {
      const fileList = Array.from(this.filesToUse.values());
      const loader = new CsvFilesLoader(fileList);
      loader.loadFiles(onComplete);
   }
}