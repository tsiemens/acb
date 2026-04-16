<template>
  <InfoDialog :store="store" dialog-id="appDescriptionDialog" title="What is ACB?">
    <p><span class="tooltip">ACB<span class="tooltiptext">
      Adjusted Cost Basis - The method in which capital gains tax are calculated in Canada. This can differ from the methods used in other countries.</span>
      </span>
      is a tool for tallying capital gains for Canadian tax returns.
    </p>
    <p>Features include:</p>
    <ul class="info-list">
      <li>Calculating <a href="https://www.canada.ca/en/revenue-agency/services/tax/individuals/topics/about-your-tax-return/tax-return/completing-a-tax-return/personal-income/line-127-capital-gains/capital-losses-deductions/what-a-superficial-loss.html">Superficial Losses</a></li>
      <li>Automatic per-day foreign exchange rate lookups (USD only supported currently)</li>
      <li>Can output to both table and text (good for saving for records)</li>
    </ul>
  </InfoDialog>

  <InfoDialog :store="store" dialog-id="fileFormatsDialog" title="Transactions CSV File Format">
    <p>Details on how to format your transactions CSV/spreadsheet can be found
      <a href="https://github.com/tsiemens/acb/wiki/Transaction-Spreadsheets">here</a>.
    </p>
    <p><a href="./samples/sample_txs.csv">Sample transactions file</a></p>
  </InfoDialog>

  <InfoDialog :store="store" dialog-id="liabilityDialog" title="Liability">
    <p>ACB is not designed or developed by certified individual, and is to be
      used <strong>at your own risk</strong>. Please cross-check any results if you have any doubts.
    </p>
  </InfoDialog>

  <InfoDialog :store="store" dialog-id="dataPolicyDialog" title="Data Policy">
    <p>ACB does not collect any personal data. All processing is done locally in your browser. No data leaves your computer.</p>
  </InfoDialog>

  <InfoDialog :store="store" dialog-id="etradeInstructionsDialog" title="E*TRADE — How to Download Your Data">
    <h4>Trade Confirmation PDFs (sell transactions)</h4>
    <ol>
      <li>Go to E*TRADE "Documents"</li>
      <li>Select your Stock Plan account</li>
      <li>Filter by "Trade Confirmations"</li>
      <li>Bulk download: Select all confirmations using the checkbox at the top, then click "Download" to get a ZIP of all PDFs</li>
      <li>Unzip and drop all PDF files here</li>
    </ol>
    <div class="info-dialog-message-warning">
      <strong>Warning:</strong> E*Trade has a 12-file download limit per batch. Trades from the same day share the same filename prefix (with _1, _2, etc. suffixes). If you split downloads across multiple batches, same-day files may overwrite each other. Verify you have one PDF per sell order — check the count against your order history.
    </div>

    <h4>Vest and ESPP confirmation PDFs</h4>
    <ol>
      <li>Go to E*TRADE "At Work", then to "Holdings"</li>
      <li>In the ESPP and RS sections, click "Benefit History"</li>
      <li>Expand each relevant section, and download (right-click and "Save link as") each "View confirmation of purchase" or "View confirmation of release" link PDF</li>
    </ol>

    <h4>BenefitHistory.xlsx (alternative vest &amp; ESPP data)</h4>
    <ol>
      <li>Go to E*TRADE Benefit History</li>
      <li>Click the + button next to each RSU grant and ESPP entry to expand all sections</li>
      <li>Once all sections are expanded, click "Download All Expanded" to export as XLSX</li>
      <li>Drop the downloaded BenefitHistory.xlsx file here</li>
    </ol>
    <div class="info-dialog-message-warning">
      <strong>Warning:</strong> This method is currently less preferred than the vest/ESPP PDFs as it interferes with the auto-USD.FX transaction generation. USD.FX "Buy"s will be generated for sell-to-covers as they are not distinguished from other manual sales.
    </div>
  </InfoDialog>

  <InfoDialog :store="store" dialog-id="questradeInstructionsDialog" title="Questrade — How to Download Your Data">
    <h4>Activities export (Activities*.xlsx)</h4>
    <ol>
      <li>Go to the Reports page</li>
      <li>Click "See all transaction history"</li>
      <li>Select the desired date range</li>
      <li>Click "Export to Excel"</li>
      <li>Drop the downloaded Activities*.xlsx file here</li>
    </ol>
  </InfoDialog>

  <InfoDialog :store="store" dialog-id="rbcDiInstructionsDialog" title="RBC Direct Investing — How to Download Your Data">
    <h4>Activity CSV export</h4>
    <ol>
      <li>Go to RBC Direct Investing</li>
      <li>Navigate to Trade &amp; Transfer &rarr; Transactions &rarr; View Activity</li>
      <li>Filter to the desired date range</li>
      <li>Click "Export"</li>
      <li>Drop the downloaded CSV file here</li>
    </ol>
  </InfoDialog>

  <InfoDialog :store="store" dialog-id="dynamicTextInfo" :title="store.dynamicTextTitle">
    <div class="dynamic-text-wrapper">
      <button class="dynamic-text-copy-btn" @click="copyDynamicText">{{ copyBtnLabel }}</button>
      <pre class="dynamic-text-content">{{ store.dynamicTextContent }}</pre>
    </div>
  </InfoDialog>

  <InfoDialog :store="store" dialog-id="tampermonkeyScriptDialog" :title="store.dynamicTextTitle" max-width="900px">
    <div class="dynamic-text-wrapper">
      <button class="dynamic-text-copy-btn" @click="copyTampermonkeyScript">{{ tmCopyBtnLabel }}</button>
      <pre class="dynamic-text-content">{{ store.dynamicTextContent }}</pre>
    </div>
    <h4 class="tm-instructions-heading">How to use:</h4>
    <ol class="tm-instructions-list">
      <li>Install the <a href="https://www.tampermonkey.net/" target="_blank" rel="noopener">TamperMonkey</a> browser extension (security settings MAY need to be changed — see note below)</li>
      <li>Click "Copy Script" above</li>
      <li>Open TamperMonkey dashboard, click the "+" tab to create a new script, clear the template, and paste</li>
      <li>Save the script (Ctrl+S / Cmd+S)</li>
      <li>Navigate to WealthSimple Tax and search for "Capital Gains" to add the <strong>Capital Gains (or Losses)</strong> section to your return</li>
      <li>A dark panel will appear pinned to the right side of the page — this is the auto-fill control panel</li>
      <li>Scroll to the Capital Gains section so it's visible, then click <strong>"Start Auto-Fill"</strong></li>
      <li>The script will automatically add rows and fill in each disposition (type, description, proceeds, cost base, expenses) — click "Stop" at any time</li>
      <li>For the 2024 tax year, transactions are automatically split into Period 1 (Jan 1 – Jun 24) and Period 2 (Jun 25 – Dec 31) based on the trade date</li>
      <li><strong>Review all values</strong> before submitting your return</li>
    </ol>
    <div class="info-dialog-message-warning tm-csp-warning">
      <p class="tm-csp-title">Important: TamperMonkey Security Settings</p>
      <p>WealthSimple Tax has strict Content Security Policy (CSP) headers that block injected scripts by default. You may need to change these TamperMonkey settings if the script panel does not appear:</p>
      <ol>
        <li>Click the TamperMonkey icon &rarr; <strong>Dashboard</strong> &rarr; <strong>Settings</strong> tab</li>
        <li>Under <strong>General</strong>, set <strong>Config Mode</strong> to <strong>"Advanced"</strong> (this reveals the security options)</li>
        <li>Scroll to <strong>Security</strong> and set <strong>"Modify existing content security policy (CSP) headers"</strong> to <strong>"Remove entirely (possibly unsafe)"</strong></li>
        <li>Click <strong>Save</strong> and reload the WealthSimple Tax page</li>
      </ol>
      <p class="tm-csp-footer">You can revert this setting after you're done filing your taxes.</p>
    </div>
  </InfoDialog>

  <InfoDialog :store="store" dialog-id="licenseDialog" title="License">
    <p>ACB is open sourced under the MIT License</p>
    <div style="font-family: monospace;">
      <p>MIT License</p>
      <p>Copyright (c) {{ copyrightYears }} Trevor Siemens</p>
      <p>
        Permission is hereby granted, free of charge, to any person obtaining a copy
        of this software and associated documentation files (the "Software"), to deal
        in the Software without restriction, including without limitation the rights
        to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
        copies of the Software, and to permit persons to whom the Software is
        furnished to do so, subject to the following conditions:
      </p>
      <p>
        The above copyright notice and this permission notice shall be included in all
        copies or substantial portions of the Software.
      </p>
      <p>
        THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
        IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
        FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
        AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
        LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
        OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
        SOFTWARE.
      </p>
    </div>
  </InfoDialog>
</template>

<script lang="ts">
import { defineComponent, ref, type PropType } from 'vue';
import type { InfoDialogStore } from './info_dialog_store.js';
import InfoDialog from './InfoDialog.vue';
import { copyrightYears } from './copyright.js';

export default defineComponent({
   name: 'InfoDialogs',
   components: { InfoDialog },
   props: {
      store: {
         type: Object as PropType<InfoDialogStore>,
         required: true,
      },
   },
   setup(props) {
      const copyBtnLabel = ref('Copy');
      const tmCopyBtnLabel = ref('Copy Script');

      function copyDynamicText() {
         navigator.clipboard.writeText(props.store.dynamicTextContent).then(() => {
            copyBtnLabel.value = 'Copied!';
            setTimeout(() => { copyBtnLabel.value = 'Copy'; }, 1500);
         }).catch(() => {
            copyBtnLabel.value = 'Failed';
            setTimeout(() => { copyBtnLabel.value = 'Copy'; }, 1500);
         });
      }

      function copyTampermonkeyScript() {
         navigator.clipboard.writeText(props.store.dynamicTextContent).then(() => {
            tmCopyBtnLabel.value = 'Copied!';
            setTimeout(() => { tmCopyBtnLabel.value = 'Copy Script'; }, 1500);
         }).catch(() => {
            tmCopyBtnLabel.value = 'Failed';
            setTimeout(() => { tmCopyBtnLabel.value = 'Copy Script'; }, 1500);
         });
      }

      return { copyrightYears: copyrightYears(), copyBtnLabel, copyDynamicText, tmCopyBtnLabel, copyTampermonkeyScript };
   },
});
</script>

<style scoped>
.info-list {
  list-style-position: inside;
  margin-left: 10px;
}

.info-list li {
  margin-bottom: 5px;
}

h4 {
  font-size: 16px;
  margin-top: 18px;
  margin-bottom: 8px;
  color: #333;
}

h4:first-child {
  margin-top: 0;
}

ol {
  margin: 0 0 12px 0;
  padding-left: 24px;
}

ol li {
  margin-bottom: 6px;
}

.info-dialog-message-warning {
  background-color: var(--warning-bg);
  border: 1px solid var(--warning-border);
  border-radius: 4px;
  padding: 10px 14px;
  margin-bottom: 15px;
  color: var(--warning-text);
  font-size: 14px;
  line-height: 1.5;
}

.dynamic-text-wrapper {
  position: relative;
}

.dynamic-text-copy-btn {
  position: absolute;
  top: 6px;
  right: 6px;
  z-index: 1;
  padding: 3px 10px;
  font-size: 11px;
  background-color: #fff;
  border: 1px solid #c8cdd3;
  border-radius: 4px;
  cursor: pointer;
  color: #444;
  transition: background-color 0.15s;
}

.dynamic-text-copy-btn:hover {
  background-color: #e9ecef;
}

.dynamic-text-content {
  margin: 0;
  padding: 10px 12px;
  background-color: #f6f8fa;
  border: 1px solid #e0e0e0;
  border-radius: 4px;
  font-size: 12px;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: 'Courier New', Courier, monospace;
  max-height: 40vh;
  overflow-y: auto;
}

.tm-instructions-heading {
  margin-top: 18px;
}

.tm-instructions-list {
  margin: 0 0 4px 0;
  padding-left: 24px;
}

.tm-instructions-list li {
  margin-bottom: 6px;
}

/* tm-csp* - TamperMonkey Content Security Policy */

.tm-csp-warning {
  margin-top: 16px;
  margin-bottom: 0;
}

.tm-csp-warning p {
  margin: 8px 0 4px;
}

.tm-csp-warning ol {
  margin: 4px 0 4px 0;
  padding-left: 24px;
}

.tm-csp-warning ol li {
  margin-bottom: 4px;
}

.tm-csp-title {
  font-weight: 600;
  margin-top: 0 !important;
  margin-bottom: 6px !important;
}

.tm-csp-footer {
  font-size: 0.9em;
  opacity: 0.75;
  margin-top: 8px !important;
  margin-bottom: 0 !important;
}
</style>
