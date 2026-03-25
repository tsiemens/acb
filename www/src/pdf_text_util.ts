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
      // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access, @typescript-eslint/no-explicit-any
      const text = content.items.map((item: any) => item.str as string).join(' ');
      pages.push(text);
   }
   return pages;
}

export async function extractTextFromPdf(data: ArrayBuffer): Promise<string> {
   const pages = await extractPdfPages(data);
   return pages.map((text, i) => `--- Page ${String(i + 1)} ---\n${text}`).join('\n\n');
}
