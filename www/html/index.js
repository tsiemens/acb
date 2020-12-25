// Map of id to File object
let filesToUse = {};
let nextFileId = 1;

function get(obj, key, def) {
   if (obj === undefined) {
      return def;
   }
   const value = obj[key];
   return (value === undefined) ? def : value;
}

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

function newElem(type, parts) {
   const elem = document.createElement(type);
   for (const clz of get(parts, "classes", [])) {
      elem.classList.add(clz);
   }
   for (const child of get(parts, "children", [])) {
      elem.appendChild(child);
   }
   const text = get(parts, 'text', undefined);
   if (text != undefined) {
      elem.innerText = text;
   }
   return elem;
}

function handleInitSecButton(button) {
   if (button.dataset.deleteOnClick) {
      const initsDiv = document.getElementById("initial-symbol-state-inputs");
      const row = button.parentElement;
      initsDiv.removeChild(row);
   } else {
      addInitialSecurityStateRow();
      button.dataset.deleteOnClick = true;
      button.innerText = "X";
   }
}

function rowHasFocus(row) {
   for (const child of row.children) {
      if (child === document.activeElement) {
         return true;
      }
   }
   return false;
}

function handleInitSecFocusChange(inp) {
   // Sleep just a bit, because when we tab to a new input, it transiently focuses
   // the document body, not the next element.
   setTimeout(() => {
      const row = inp.parentElement;
      if (!rowHasFocus(row)) {
         validateInitSecRow(row);
      }
   }, 100);
}

function getRowContents(row) {
   return {
      secInput: row.getElementsByClassName("init-sec-name")[0],
      secQuantInput: row.getElementsByClassName("init-sec-quant")[0],
      secAcbInput: row.getElementsByClassName("init-sec-acb")[0]
   };
}

function validateInitSecRow(rowElem) {
   const row = getRowContents(rowElem);

   const setError = (elem, err) => {
      if (err) {
         elem.classList.add("init-sec-input-error");
      } else {
         elem.classList.remove("init-sec-input-error");
      }
   };

   if (row.secInput.value) {
      setError(row.secInput, false);
      setError(row.secQuantInput, row.secQuantInput.value == false);
      setError(row.secAcbInput, row.secAcbInput.value == false);
   } else if (!row.secInput.value && (row.secQuantInput.value || row.secAcbInput.value)) {
      setError(row.secInput, true);
      setError(row.secQuantInput, false);
      setError(row.secAcbInput, false);
   } else {
      setError(row.secInput, false);
      setError(row.secQuantInput, false);
      setError(row.secAcbInput, false);
   }
}

function getInitSecs() {
   const initsDiv = document.getElementById("initial-symbol-state-inputs");
   valids = [];
   invalids = [];
   for (const rowElem of initsDiv.children) {
      const row = getRowContents(rowElem);
      const security = row.secInput.value;
      const quant = row.secQuantInput.value;
      const acb = row.secAcbInput.value;
      if (security) {
         if (quant && acb) {
            valids.push(security + ":" + quant + ":" + acb);
         } else {
            invalids.push("Invalid quantity and/or ACB for " + security);
         }
      } else if (quant || acb) {
         invalids.push("Missing security name");
      }
   }

   const errorsElem = document.getElementById("init-secs-errors");
   if (invalids) {
      errorsElem.innerText = invalids.join("\n");
   } else {
      errorsElem.innerText = "";
   }

   return {"valid": valids, "invalid": invalids}
}

function addInitialSecurityStateRow() {
   const initsDiv = document.getElementById("initial-symbol-state-inputs");

   const newInitDiv = newElem("div", {classes: ["init-sec-row"]});
   newInitDiv.innerHTML = `
         <input type="text" class="init-sec-input init-sec-name"
                onfocus="handleInitSecFocusChange(this)"
                onfocusout="handleInitSecFocusChange(this)"
                placeholder="SECURITY"/>
         <input type="number" class="init-sec-input init-sec-quant"
                onfocus="handleInitSecFocusChange(this)"
                onfocusout="handleInitSecFocusChange(this)"
                placeholder="quantity" pattern="[0-9]+"/>
         <input type="number" class="init-sec-input init-sec-acb"
                onfocus="handleInitSecFocusChange(this)"
                onfocusout="handleInitSecFocusChange(this)"
                placeholder="total cost basis (CAD)"/>
         <button class="init-sec-button" onclick="handleInitSecButton(this)"
                 onfocus="handleInitSecFocusChange(this)"
                 onfocusout="handleInitSecFocusChange(this)">
            Add</button>`;

   initsDiv.appendChild(newInitDiv);
}

function populateTables(model) {
   if (model === undefined) {
      model = {"STOCK": {
         "footer": ["", "", "", "", "", "", "", "Total", "$0", "", "", "", "", ""],
         "header": ["Security", "Date", "TX", "Amount", "Shares", "Amt/Share", "ACB",
                    "Commission", "Cap. Gain", "Share Balance", "ACB +/-", "New ACB",
                     "New ACB/Share", "Memo"],
         "rows": [],
      }};
   }

   const tablesContainer = document.getElementById("acb-table-output");
   tablesContainer.innerHTML = "";

   const symbols = Object.keys(model);
   symbols.sort()
   for (const symbol of symbols) {
      const symModel = model[symbol];

      const tr = newElem("tr");
      for (const header of symModel.header) {
         tr.appendChild(newElem("th", {text: header}));
      }

      const tbody = newElem('tbody');

      const addRow = function(rowItems) {
         const rowElem = newElem('tr');
         for (const item of rowItems) {
            const td = newElem('td', {text: item});
            rowElem.appendChild(td);
         }
         tbody.appendChild(rowElem);
      };

      for (const row of symModel.rows) {
         addRow(row);
      }
      addRow(symModel.footer);

      const table = newElem('table', {
         children: [newElem('thead', {children:[tr]}), tbody]
      });

      const symTableContainer = newElem("div", {
         classes: ['table-fixed-head'],
         children: [table]
      });

      tablesContainer.appendChild(
         newElem('div', {classes: ['symbol-name'], text: symbol}));

      const errors = get(symModel, 'errors', []);
      for (const err of errors) {
         tablesContainer.appendChild(newElem('p', {classes: ['error-text'], text: err}));
      }
      if (errors.length > 0) {
         tablesContainer.appendChild(newElem('p', {text: "Information is of parsed state only, and may not be fully correct."}));
      }
      tablesContainer.appendChild(symTableContainer);
      for (const note of get(symModel, 'notes', [])) {
         tablesContainer.appendChild(newElem('p', {text: note}));
      }
   }
}

function showTableOut() {
   setTabActive('table');
}

function showTextOut() {
   setTabActive('text');
}

function setTabActive(labelStr) {
   const tabLabels = document.getElementsByClassName('tab-label');
   for (const tabLabel of tabLabels) {
      if (tabLabel.dataset.tabLabel === labelStr) {
         tabLabel.classList.add('active');
      } else {
         tabLabel.classList.remove('active');
      }
   }

   const textOutput = document.getElementById('acb-text-output');
   const tableOutput = document.getElementById('acb-table-output');

   if (labelStr == 'text') {
      textOutput.classList.remove('inactive');
      tableOutput.classList.add('inactive');
   } else if (labelStr == 'table') {
      textOutput.classList.add('inactive');
      tableOutput.classList.remove('inactive');
   }
}

async function asyncRunAcb(filenames, contents) {
   superficialLosses = document.getElementById('superficial-losses-checkbox').checked;
   const initSecs = getInitSecs();
   if (initSecs.invalid.length) {
      return;
   }
   const ret = runAcb(filenames, contents, initSecs.valid, superficialLosses);
   try {
      const resp = await ret;
      let error = resp.error;
      console.log("asyncRunAcb response received" +
                  (error === undefined ? "" : " with error"));
      const acbOutElem = document.getElementById("acb-text-output");
      acbOutElem.innerText = resp.result ? resp.result.textOutput : "";
      const errorsElem = document.getElementById("acb-errors");
      if (error !== undefined) {
         errorsElem.innerText = error;
      } else {
         errorsElem.innerText = "";
      }

      populateTables(resp ? resp.result.modelOutput : {});
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
   const initSecs = getInitSecs();
   if (initSecs.invalid.length) {
      return;
   }

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
      }
      loadAllFileInfoAndRun(fileList);
   });

   addInitialSecurityStateRow();

   showTableOut();

   // Return objects that need to stay alive.
   return {"go": go}
}
