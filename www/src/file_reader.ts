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

// Decodes a base64 string to a UTF-8 string, handling BOM if present.
// (In base 64, BOM is "77u/" at the start of the string.)
function decodeBase64(base64String: string) {
   const binaryString = atob(base64String);
   const bytes = new Uint8Array(binaryString.length);

   for (let i = 0; i < binaryString.length; i++) {
      bytes[i] = binaryString.charCodeAt(i);
   }

   // TextDecoder automatically handles  UTF-8 Byte Order Mark (BOM) removal with
   // the ignoreBOM option
   const decoder = new TextDecoder('utf-8', { ignoreBOM: true });
   return decoder.decode(bytes);
}

export function fileBytesToString(bytes: Uint8Array): string {
   // If result is an ArrayBuffer, convert it to a string.
   const decoder = new TextDecoder("utf-8");
   const resultStr: string = decoder.decode(bytes);

   // Decode base64 if applicable.
   const b64Parts = resultStr.split(";base64,");
   let content: string;
   let b64Decoded = false;
   if (b64Parts.length < 2) {
      // Not a base64 string, return as-is.
      content = resultStr;
   } else {
      content = decodeBase64(b64Parts[1]);
      b64Decoded = true;
   }
   if (content && content.length > 100) {
      console.debug(`fileBytesToString (b64: ${b64Decoded}): "${content.slice(0, 50)}" ... ` +
                     `"${content.slice(-50)}`);
   } else {
      console.debug(`fileBytesToString: (b64: ${b64Decoded})`, content);
   }
   return content;
}

export interface FileByteResult {
   name: string;
   data: Uint8Array;
   error?: string;
}

// Loads an array of files as raw bytes (Uint8Array). Suitable for both text and
// binary files. Calls onComplete once all files have been read.
export function loadFilesAsBytes(
   files: File[],
   onComplete: (results: FileByteResult[]) => void,
): void {
   if (files.length === 0) {
      onComplete([]);
      return;
   }

   const results: FileByteResult[] = files.map((f) => ({ name: f.name, data: new Uint8Array(0) }));
   let remaining = files.length;

   files.forEach((file, i) => {
      const reader = new FileReader();
      reader.addEventListener('loadend', (event) => {
         remaining--;
         const result = event.target?.result;
         if (result instanceof ArrayBuffer) {
            results[i].data = new Uint8Array(result);
         } else {
            results[i].error = event.target?.error
               ? `${event.target.error.name}: ${event.target.error.message}`
               : 'Unknown error reading file';
         }
         if (remaining === 0) onComplete(results);
      });
      reader.readAsArrayBuffer(file);
   });
}
