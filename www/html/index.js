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

function readCsv(file, targetElem) {
  // Check if the file is an image.
  if (file.type && file.type.indexOf('text/csv') === -1) {
    console.log('File is not a csv.', file.type, file);
    return;
  }

  const reader = new FileReader();
  reader.addEventListener('load', (event) => {
     console.log(event.target.result);
     const b64Content = event.target.result.split(";base64,")[1];
     const content = atob(b64Content);
     // Decode base64
     targetElem.innerText = content;

     // Golang function
     runAcb([file.name], [content]);
  });
  reader.readAsDataURL(file);
}

function loadAllFileInfo(files) {
   // Takes a list of File
   const fileDetailsList = document.getElementById("file-details-list");
   fileDetailsList.innerHTML = "";
   for (const file of files) {
      if (file.type == "text/csv") {
         fileDetailsList.innerHTML += "<li>" + file.name + "</li>";
         readCsv(file, document.getElementById("csv-content"));
      } else {
         fileDetailsList.innerHTML += "<li>" + file.name + " (Ignored. Not CSV)</li>";
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

      // const fileDetailsList = document.getElementById("file-details-list");
      // fileDetailsList.innerHTML = "";
      // for (const fileData of fileDatas) {
         // if (fileData.type == "text/csv") {
            // fileDetailsList.innerHTML += "<li>" + fileData.name + "</li>";
            // readCsv(fileData.file, document.getElementById("csv-content"));
         // } else {
            // fileDetailsList.innerHTML += "<li>" + fileData.name + " (Ignored. Not CSV)</li>";
         // }
      // }
   });

   // Return objects that need to stay alive.
   return {"go": go}
}
