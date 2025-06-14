import { mustGet, get } from "./basic_utils.js";

// Replication of AppResultOk in app_shim.rs:

export class RenderTable {
   constructor(
      public header: Array<string>,
      public rows: Array<Array<string>>,
      public footer: Array<string>,
      public notes?: Array<string>,
      public errors?: Array<string>,
   ) {}

   /* Import type from wasm interface */
   // eslint-disable-next-line @typescript-eslint/no-explicit-any
   public static fromJsValue(val: any): RenderTable {
      return new RenderTable(
         mustGet(val, 'header'),
         mustGet(val, 'rows'),
         mustGet(val, 'footer'),
         get(val, 'notes') || undefined,
         get(val, 'errors') || undefined,
      );
   }
}

export class AppRenderResult {
   constructor(
      public securityTables: Map<string, RenderTable>,
      public aggregateGainsTable: RenderTable,
   ) {}

   /* Import type from wasm interface */
   // eslint-disable-next-line @typescript-eslint/no-explicit-any
   public static fromJsValue(val: any): AppRenderResult {
      let secTables = new Map<string, RenderTable>();
      // Object map of security -> render table
      let inSecTables: {[key: string]: unknown} = mustGet(val, 'securityTables');
      console.debug("AppRenderResult.fromJsValue inSecTables", inSecTables);
      if (inSecTables instanceof Map) {
         for (const [sec, table] of inSecTables) {
            secTables.set(
               sec as string,
               RenderTable.fromJsValue(table)
            );
         }
      } else {
         for (const [sec, table] of Object.entries(inSecTables)) {
            secTables.set(
               sec,
               RenderTable.fromJsValue(table)
            );
         }
      }
      let aggregateGainsTable =
         RenderTable.fromJsValue(mustGet(val, 'aggregateGainsTable'));

      console.debug("AppRenderResult.fromJsValue secTables", secTables);
      console.debug("AppRenderResult.fromJsValue aggregateGainsTable", aggregateGainsTable);
      return new AppRenderResult(
         secTables,
         aggregateGainsTable,
      );
   }

   public static default(): AppRenderResult {
      return AppRenderResult.fromJsValue({
            "securityTables": {
               "STOCK": {
                  "footer": ["", "", "", "", "", "", "", "Total", "$0", "", "", "", "", ""],
                  "header": ["Security", "Date", "TX", "Amount", "Shares", "Amt/Share", "ACB",
                              "Commission", "Cap. Gain", "Share Balance", "ACB +/-", "New ACB",
                              "New ACB/Share", "Memo"],
                  "rows": [],
               }
            },
            "aggregateGainsTable": {
               "footer": [],
               "header": ["Year", "Capital Gains"],
               "rows": [],
            }
         });
   }
}

export class AppResultOk {
    constructor(
        public textOutput: string,
        public modelOutput: AppRenderResult,
    ) {}

    /* Import type from wasm interface */
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    public static fromJsValue(val: any): AppResultOk {
        return new AppResultOk(
            mustGet(val, 'textOutput'),
            AppRenderResult.fromJsValue(mustGet(val, 'modelOutput')),
        );
    }

   public static default(): AppResultOk {
      return new AppResultOk(
            "",
            AppRenderResult.default(),
      );
   }
}

export class FileContent {
    constructor(
        public fileName: string,
        public content: string,
    ) {}

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    public static fromJsValue(val: any): FileContent {
        return new FileContent(
        mustGet(val, 'fileName'),
        mustGet(val, 'content'),
        );
    }
}

export class AppExportResultOk {
    constructor(
        public csvFiles: Array<FileContent>,
    ) {}

    /* Import type from wasm interface */
    /* eslint-disable-next-line @typescript-eslint/no-explicit-any */
    public static fromJsValue(val: any): AppExportResultOk {
        return new AppExportResultOk(
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            mustGet<any[]>(val, 'csvFiles').map(FileContent.fromJsValue),
        );
    }

   public static default(): AppExportResultOk {
      return new AppExportResultOk(
            [],
      );
   }
}