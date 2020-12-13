// Map of id to File object
let filesToUse = {};
let nextFileId = 1;

function setRunButtonEnabled(enabled) {
   const runButton = document.getElementById('run-button');
   runButton.disabled = !enabled;
}

function addFileToUse(file) {
   const fileId = nextFileId;
   nextFileId++;
   filesToUse[fileId] = file;
   return fileId;
}

function removeFileListEntry(fileId) {
   const fileList = document.getElementsByClassName("file-list")[0];
   for (const child of fileList.children) {
      if (child.dataset.fileid == fileId) {
         fileList.removeChild(child);
      }
   }

   if (fileList.children.length == 0) {
      setRunButtonEnabled(false);
   }
}

function addFileListEntry(fileId, filename) {
   const fileList = document.getElementsByClassName("file-list")[0];
   const entry = document.createElement('div');
   entry.classList.add('file-list-item');
   const btn = document.createElement('button');
   btn.innerText = 'X';
   btn.addEventListener("click", (event) => {
      const fildId = event.target.dataset.fileid;
      console.log("Click X button for fileId", fileId);
      delete filesToUse[fileId];
      removeFileListEntry(fileId);
   });

   entry.appendChild(btn);
   const entryText = document.createElement('div');
   entryText.classList.add('file-list-item-text');
   entryText.innerText = ' ' + filename;
   entry.appendChild(entryText);

   entry.dataset.fileid = fileId;

   fileList.appendChild(entry);

   if (fileList.children.length > 0) {
      setRunButtonEnabled(true);
   }
}

function isFileAlreadySelected(file) {
   for (const fileId in filesToUse) {
      const selFile = filesToUse[fileId];
      if (selFile.name == file.name && selFile.lastModified == file.lastModified) {
         return true;
      }
   }
   return false;
}

function addFilesToUse(fileList) {
   printMetadataForFileList(fileList);
   for (const file of fileList) {
      if (file.type == "text/csv") {
         if (isFileAlreadySelected(file)) {
            console.log("File", file.name, "already selected.");
         } else {
            const fileId = addFileToUse(file);
            addFileListEntry(fileId, file.name);
         }
      } else {
         console.log("File " + file.name + " ignored. Not CSV.");
      }
   }
}

function getRequestedFileNames() {
   const fileNames = [];
   const fileEntries = document.getElementsByClassName("file-list-item");
   for (const entry of fileEntries) {
      fileNames.push(entry.dataset.filename);
   }
   return fileNames;
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
      let error = resp.error;
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

function loadAllFileInfoAndRun(files) {
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
       console.log("wasm instantiation complete");
   });

   const dropArea = document.getElementById('file-drop-area');
   const dropAreaOuter = document.getElementById('file-drop-area-outer');

   dropArea.addEventListener('dragover', (event) => {
      event.stopPropagation();
      event.preventDefault();
      // Style the drag-and-drop as a "copy file" operation.
      event.dataTransfer.dropEffect = 'copy';
      dropArea.setAttribute("drop-active", true);
      dropAreaOuter.setAttribute("drop-active", true);
   });

   dropArea.addEventListener('dragleave', (event) => {
      dropArea.setAttribute("drop-active", false);
      dropAreaOuter.setAttribute("drop-active", false);
   });

   dropArea.addEventListener('drop', (event) => {
      event.stopPropagation();
      event.preventDefault();
      dropArea.setAttribute("drop-active", false);
      dropAreaOuter.setAttribute("drop-active", false);
      const fileList = event.dataTransfer.files;
      addFilesToUse(fileList);
   });

   const fileSelector = document.getElementById('file-selector-input');
   fileSelector.addEventListener('input', (event) => {
      const fileList = event.target.files;
      console.log("on input:", fileList);
      addFilesToUse(fileList);
   });

   const runButton = document.getElementById('run-button');
   runButton.disabled = true;
   runButton.addEventListener('click', (event) => {
      const fileList = [];
      for (const fileId in filesToUse) {
         fileList.push(filesToUse[fileId]);
         loadAllFileInfoAndRun(fileList);
      }
   });

   // Return objects that need to stay alive.
   return {"go": go}
}
