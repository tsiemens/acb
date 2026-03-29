<template>
  <InfoDialogBackdrop :store="store" />

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
    <p>ACB does not collect any personal data. All processing is done locally to your browser.</p>
  </InfoDialog>

  <InfoDialog :store="store" dialog-id="dynamicTextInfo" :title="store.dynamicTextTitle">
    <pre class="dynamic-text-content">{{ store.dynamicTextContent }}</pre>
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
import { defineComponent, type PropType } from 'vue';
import type { InfoDialogStore } from './info_dialog_store.js';
import InfoDialog from './InfoDialog.vue';
import InfoDialogBackdrop from './InfoDialogBackdrop.vue';
import { copyrightYears } from './copyright.js';

export default defineComponent({
   name: 'InfoDialogs',
   components: { InfoDialog, InfoDialogBackdrop },
   props: {
      store: {
         type: Object as PropType<InfoDialogStore>,
         required: true,
      },
   },
   setup() {
      return { copyrightYears: copyrightYears() };
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
  max-height: 60vh;
  overflow-y: auto;
}
</style>
