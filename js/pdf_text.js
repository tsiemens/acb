/**
 * PDF text extraction using pdfjs-dist.
 *
 * This script is used internally by the crate's pdf module as an alternative
 * PDF reader engine.
 *
 * Usage: node pdf_text.js <filename> [options]
 *   -m, --parsable-page-markers   Insert PAGE_BREAK<N> markers between pages
 *   -n, --show-page-numbers       Print readable page delimiters
 *   -p, --page <number>           Extract specific pages (can be repeated)
 *
 * Requires NODE_PATH to be set to the node_modules directory containing pdfjs-dist.
 */

const fs = require("fs");
const path = require("path");

function parseArgs(argv) {
    const args = {
        fname: null,
        showPageNumbers: false,
        parsablePageMarkers: false,
        pages: null,
    };

    let i = 2; // skip node and script path
    while (i < argv.length) {
        const arg = argv[i];
        if (arg === "-m" || arg === "--parsable-page-markers") {
            args.parsablePageMarkers = true;
        } else if (arg === "-n" || arg === "--show-page-numbers") {
            args.showPageNumbers = true;
        } else if (arg === "-p" || arg === "--page") {
            i++;
            if (i >= argv.length) {
                process.stderr.write("Error: --page requires a value\n");
                process.exit(1);
            }
            if (!args.pages) args.pages = [];
            args.pages.push(parseInt(argv[i], 10));
        } else if (!arg.startsWith("-")) {
            args.fname = arg;
        } else {
            process.stderr.write(`Unknown argument: ${arg}\n`);
            process.exit(1);
        }
        i++;
    }

    if (!args.fname) {
        process.stderr.write("Error: filename is required\n");
        process.exit(1);
    }

    return args;
}

async function main() {
    const args = parseArgs(process.argv);

    // pdfjs-dist is available via NODE_PATH set by the Rust caller
    const pdfjsLib = require("pdfjs-dist/legacy/build/pdf.mjs");

    const data = new Uint8Array(fs.readFileSync(args.fname));
    const doc = await pdfjsLib.getDocument({ data }).promise;

    for (let pageNum = 1; pageNum <= doc.numPages; pageNum++) {
        if (args.pages && !args.pages.includes(pageNum)) {
            continue;
        }

        const page = await doc.getPage(pageNum);
        const textContent = await page.getTextContent();

        // Reconstruct text from items, preserving line structure
        let lastY = null;
        let text = "";
        for (const item of textContent.items) {
            if (!item.str && item.str !== "") continue;
            const y = item.transform ? item.transform[5] : null;
            if (lastY !== null && y !== null && Math.abs(y - lastY) > 1) {
                text += "\n";
            }
            text += item.str;
            lastY = y;
        }

        if (args.parsablePageMarkers) {
            process.stdout.write(`PAGE_BREAK<${pageNum}>`);
        } else if (args.showPageNumbers) {
            if (pageNum > 1) {
                process.stdout.write("\n");
            }
            process.stdout.write(`---------- Page ${pageNum} ----------\n`);
        }

        if (args.parsablePageMarkers) {
            process.stdout.write(text);
        } else {
            process.stdout.write(text + "\n");
        }
    }
}

main().catch((err) => {
    process.stderr.write(`Error: ${err.message}\n`);
    process.exit(1);
});
