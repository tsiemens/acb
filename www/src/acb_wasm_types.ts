import { mustGet, get } from "./basic_utils.js";

// -- FX Rates Cache interchange types --

export interface SerializableDailyRate {
   date: string;
   rate: string;
}

export interface SerializableYearRates {
   year: number;
   rates: SerializableDailyRate[];
}

export interface RatesCacheData {
   years: SerializableYearRates[];
}

export interface RatesCacheUpdate {
   years: SerializableYearRates[];
}

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
        public ratesCacheUpdate?: RatesCacheUpdate,
    ) {}

    /* Import type from wasm interface */
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    public static fromJsValue(val: any): AppResultOk {
        return new AppResultOk(
            mustGet(val, 'textOutput'),
            AppRenderResult.fromJsValue(mustGet(val, 'modelOutput')),
            get(val, 'ratesCacheUpdate') as RatesCacheUpdate | undefined,
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
        public securitiesWithErrors: Array<string>,
        public ratesCacheUpdate?: RatesCacheUpdate,
    ) {}

    /* Import type from wasm interface */
    /* eslint-disable-next-line @typescript-eslint/no-explicit-any */
    public static fromJsValue(val: any): AppExportResultOk {
        return new AppExportResultOk(
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            mustGet<any[]>(val, 'csvFiles').map(FileContent.fromJsValue),
            mustGet<string[]>(val, 'securitiesWithErrors'),
            get(val, 'ratesCacheUpdate') as RatesCacheUpdate | undefined,
        );
    }

   public static default(): AppExportResultOk {
      return new AppExportResultOk(
            [],
            [],
      );
   }
}

export class AppSummaryResultOk {
    constructor(
        public csvText: string,
        public summaryTable: RenderTable,
        public ratesCacheUpdate?: RatesCacheUpdate,
    ) {}

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    public static fromJsValue(val: any): AppSummaryResultOk {
        return new AppSummaryResultOk(
            mustGet(val, 'csvText'),
            RenderTable.fromJsValue(mustGet(val, 'summaryTable')),
            get(val, 'ratesCacheUpdate') as RatesCacheUpdate | undefined,
        );
    }

    public static default(): AppSummaryResultOk {
        return new AppSummaryResultOk(
            '',
            new RenderTable([], [], [], [], []),
        );
    }
}

