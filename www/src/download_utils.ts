import JSZip from "jszip";
import { asError } from "./http_utils.js";
import { ErrorBox } from "./vue/error_box_store.js";
import type { FileEntry } from "./vue/file_manager_store.js";
import type { FileContent } from "./acb_wasm_types.js";

export function makeFilenameDateString(): string {
   let date_str = new Date().toISOString();
   // Replace colons and dots for filename safety
   date_str = date_str.replace(/[:.]/g, "-");
   return date_str;
}

export function downloadBlob(filename: string, blob: Blob) {
   // Create a temporary link to trigger the download
   const url = URL.createObjectURL(blob);
   const a = document.createElement("a");
   a.href = url;
   a.style.display = "none";
   a.download = filename;
   document.body.appendChild(a);
   a.click();
   // Clean up the URL object
   document.body.removeChild(a);
   URL.revokeObjectURL(url);
}

export function makeZip(files: FileContent[]): Promise<Blob> {
   return new Promise((resolve, reject) => {
      try {
         const zip = new JSZip();
         for (const file of files) {
            zip.file(file.fileName, file.content);
         }
         zip.generateAsync({ type: "blob" })
            .then(resolve)
            .catch(reject);
      } catch (error) {
         reject(asError(error));
      }
   });
}

export function makeZipAndDownload(files: FileContent[]): void {
   makeZip(files).then((zipBlob) => {
      const date_str = makeFilenameDateString();
      const filename = `acb_export_${date_str}.zip`;
      downloadBlob(filename, zipBlob);
   }).catch((err: unknown) => {
      console.error("Error creating zip file: ", err);
      ErrorBox.getMain().showWith({
         title: "Export Error",
         descPre: "An error occurred while creating the export zip file:",
         errorText: String(err),
      });
   });
}

export function downloadCsv(filenameBase: string, csvContent: string) {
   const date_str = makeFilenameDateString();
   const filename = `${filenameBase}_${date_str}.csv`;
   const blob = new Blob([csvContent], { type: "text/csv" });
   downloadBlob(filename, blob);
}

export function downloadSelectedFiles(files: FileEntry[]): void {
   if (files.length === 0) return;

   if (files.length === 1) {
      const file = files[0];
      const blob = new Blob([new Uint8Array(file.data)]);
      downloadBlob(file.name, blob);
      return;
   }

   const zip = new JSZip();
   for (const file of files) {
      zip.file(file.name, new Uint8Array(file.data));
   }
   zip.generateAsync({ type: "blob" }).then((zipBlob) => {
      const date_str = makeFilenameDateString();
      downloadBlob(`acb_files_${date_str}.zip`, zipBlob);
   }).catch((err: unknown) => {
      console.error("Error creating zip file: ", err);
      ErrorBox.getMain().showWith({
         title: "Download Error",
         descPre: "An error occurred while creating the download zip file:",
         errorText: String(err),
      });
   });
}
