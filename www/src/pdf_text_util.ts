import * as pdfjsLib from 'pdfjs-dist';

let workerInitialized = false;

async function ensureWorker(): Promise<void> {
   if (workerInitialized) return;
   const mod = await import('pdfjs-dist/build/pdf.worker.min.mjs?url');
   pdfjsLib.GlobalWorkerOptions.workerSrc = mod.default;
   workerInitialized = true;
}

export async function extractPdfPages(data: ArrayBuffer): Promise<string[]> {
   await ensureWorker();
   const pdf = await pdfjsLib.getDocument({ data }).promise;
   const pages: string[] = [];
   for (let i = 1; i <= pdf.numPages; i++) {
      const page = await pdf.getPage(i);
      const content = await page.getTextContent();
      // Reconstruct text from items, preserving line structure by
      // detecting Y-coordinate changes (matching js/pdf_text.js logic).
      let lastY: number | null = null;
      let text = '';
      for (const item of content.items) {
         // eslint-disable-next-line @typescript-eslint/no-explicit-any
         const ti = item as any;
         // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
         if (!ti.str && ti.str !== '') continue;
         // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
         const y: number | null = ti.transform ? (ti.transform[5] as number) : null;
         if (lastY !== null && y !== null && Math.abs(y - lastY) > 1) {
            text += '\n';
         }
         // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
         text += ti.str as string;
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
