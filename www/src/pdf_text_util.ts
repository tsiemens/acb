import * as pdfjsLib from 'pdfjs-dist';

let sharedWorker: pdfjsLib.PDFWorker | null = null;

let workerReady: Promise<pdfjsLib.PDFWorker> | null = null;

function ensureWorker(): Promise<pdfjsLib.PDFWorker> {
   if (!workerReady) {
      workerReady = import('pdfjs-dist/build/pdf.worker.min.mjs?url').then((mod) => {
         pdfjsLib.GlobalWorkerOptions.workerSrc = mod.default;
         sharedWorker = new pdfjsLib.PDFWorker();
         return sharedWorker;
      });
   }
   return workerReady;
}

export async function extractPdfPages(data: ArrayBuffer): Promise<string[]> {
   const worker = await ensureWorker();
   const pdf = await pdfjsLib.getDocument({ data, worker }).promise;
   const pages: string[] = [];
   for (let i = 1; i <= pdf.numPages; i++) {
      const page = await pdf.getPage(i);
      const content = await page.getTextContent();
      // Reconstruct text from items, preserving line structure by
      // detecting Y-coordinate changes (matching js/pdf_text.js logic).
      let lastY: number | null = null;
      let text = '';
      for (const item of content.items) {
         const ti = item as { str?: string; transform?: number[] };
         if (!ti.str && ti.str !== '') continue;
         const y: number | null = ti.transform ? ti.transform[5] : null;
         if (lastY !== null && y !== null && Math.abs(y - lastY) > 1) {
            text += '\n';
         }
         text += ti.str;
         lastY = y;
      }
      pages.push(text);
   }
   return pages;
}

export async function extractTextFromPdf(data: ArrayBuffer): Promise<string> {
   const pages = await extractPdfPages(data);
   return pages.map((text, i) => `--- Page ${String(i + 1)} ---\n${text}`).join('\n\n');
}
