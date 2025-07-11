#!/usr/bin/env node

const fs = require('fs');
const pdfParse = require('pdf-parse');

async function extractText(pdfPath, pageNumbers = null) {
    try {
        const pdfBuffer = fs.readFileSync(pdfPath);
        const data = await pdfParse(pdfBuffer);
        // pdf-parse returns all text as a single string, with page breaks as '\f'
        const allPages = data.text.split('\f');
        // Remove any trailing empty page
        const pages = allPages.map(s => s.trimEnd()).filter((s, i, arr) => !(i === arr.length - 1 && s === ''));

        const pagesToProcess = pageNumbers ?
            pageNumbers.filter(p => p > 0 && p <= pages.length) :
            Array.from({length: pages.length}, (_, i) => i + 1);

        let result = '';
        for (const pageNum of pagesToProcess) {
            const pageText = pages[pageNum - 1] || '';
            result += `PAGE_BREAK<${pageNum}>\n${pageText}\n`;
        }
        console.log(result);
    } catch (error) {
        console.error(error);
        process.exit(1);
    }
}

function parseArgs() {
    const args = process.argv.slice(2);
    let parsablePageMarkers = false;
    let pageNumbers = [];
    let pdfPath = null;

    for (let i = 0; i < args.length; i++) {
        if (args[i] === '--parsable-page-markers') {
            parsablePageMarkers = true;
        } else if (args[i] === '-p' && i + 1 < args.length) {
            pageNumbers.push(parseInt(args[++i], 10));
        } else if (!pdfPath) {
            pdfPath = args[i];
        }
    }

    if (!pdfPath) {
        console.error('PDF path is required');
        process.exit(1);
    }

    return {
        parsablePageMarkers,
        pageNumbers: pageNumbers.length > 0 ? pageNumbers : null,
        pdfPath
    };
}

const { parsablePageMarkers, pageNumbers, pdfPath } = parseArgs();
extractText(pdfPath, pageNumbers).catch(console.error);
