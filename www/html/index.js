function doFormat() {
   textArea = document.getElementById("json-text-area")
   // FormatJSON is from the wasm module
   formatted = formatJSON(textArea.value)
   outTextArea = document.getElementById("formatted-json-text-area")
   outTextArea.value = formatted
}

function readImage(file) {
  // Check if the file is an image.
  if (file.type && file.type.indexOf('image') === -1) {
    console.log('File is not an image.', file.type, file);
    return;
  }

  const reader = new FileReader();
  reader.addEventListener('load', (event) => {
    img.src = event.target.result;
  });
  reader.readAsDataURL(file);
}

function printMetadataForFileList(fileList) {
   for (const file of fileList) {
      // Not supported in Safari for iOS.
      const name = file.name ? file.name : 'NOT SUPPORTED';
      // Not supported in Firefox for Android or Opera for Android.
      const type = file.type ? file.type : 'NOT SUPPORTED';
      // Unknown cross-browser support.
      const size = file.size ? file.size : 'NOT SUPPORTED';
      data = {file, name, type, size};
      console.log(data);
   }
}

async function asyncRunAcb(filenames, contents) {
   const ret = runAcb(filenames, contents);
   try {
      const resp = await ret.result;
      var error = resp.error;
      console.log("asyncRunAcb response received" +
                  (error === undefined ? "" : " with error"));
      const acbOutElem = document.getElementById("acb-text-output");
      acbOutElem.innerText = resp.result;
      const errorsElem = document.getElementById("acb-errors");
      if (error !== undefined) {
         errorsElem.innerText = error;
      } else {
         errorsElem.innerText = "";
      }
   } catch (err) {
      console.log("asyncRunAcb caught error: ", err);
   }
}

/**
 * Takes a File, Dom element to write into, and a FileLoadQueue.
 */
function readCsv(file, loadQueue) {
  // Check if the file is an image.
  if (file.type && file.type.indexOf('text/csv') === -1) {
    console.log('File is not a csv.', file.type, file);
    return;
  }

  const reader = new FileReader();
  reader.addEventListener('load', (event) => {
     console.log(event.target.result);
     // Decode base64
     const b64Content = event.target.result.split(";base64,")[1];
     const content = atob(b64Content);

     const queueIdx = loadQueue.pendingFileNames.indexOf(file.name);
     if (queueIdx >= 0) {
        loadQueue.pendingFileNames.splice(queueIdx, 1);
        loadQueue.loadedContent.push(content);
        loadQueue.loadedFileNames.push(file.name);
     }

     if (loadQueue.pendingFileNames.length == 0) {
        // Golang function
        asyncRunAcb(loadQueue.loadedFileNames, loadQueue.loadedContent);
     }
  });
  reader.readAsDataURL(file);
}

class FileLoadQueue {
  constructor(pendingFileNames) {
     this.pendingFileNames = pendingFileNames;
     this.loadedContent = [];
     this.loadedFileNames = [];
  }
}

function loadAllFileInfo(files) {
   // Takes a list of File
   fileNames = [];
   for (const file of files) {
      fileNames.push(file.name);
   }
   loadQueue = new FileLoadQueue(fileNames);

   for (const file of files) {
      if (file.type == "text/csv") {
         console.log("Loading file: " + file.name);
         readCsv(file, loadQueue);
      } else {
         console.log("File " + file.name + " ignored. Not CSV.");
      }
   }
}

function initPageJs() {
   const go = new Go();
   WebAssembly.instantiateStreaming(fetch("wasm/acb.wasm"), go.importObject).then((result) => {
       go.run(result.instance);
       console.log(golangDemo("Foo"));
   });

   // const fileSelector = document.getElementById('file-selector');
   // fileSelector.addEventListener('change', (event) => {
      // const fileList = event.target.files;
      // console.log(fileList);
   // });

   const dropArea = document.getElementById('file-drop-area');

   dropArea.addEventListener('dragover', (event) => {
      event.stopPropagation();
      event.preventDefault();
      // Style the drag-and-drop as a "copy file" operation.
      event.dataTransfer.dropEffect = 'copy';
      event.target.setAttribute("drop-active", true);
   });

   dropArea.addEventListener('dragleave', (event) => {
      event.target.setAttribute("drop-active", undefined);
   });

   dropArea.addEventListener('drop', (event) => {
      event.stopPropagation();
      event.preventDefault();
      event.target.setAttribute("drop-active", undefined);
      const fileList = event.dataTransfer.files;
      console.log(fileList);
      printMetadataForFileList(fileList);
      loadAllFileInfo(fileList);
   });

   // Return objects that need to stay alive.
   return {"go": go}
}
