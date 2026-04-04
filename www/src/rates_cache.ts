import { ref } from "vue";
import { RatesCacheUpdate, RatesCacheData } from "./acb_wasm_types.js";

const STORAGE_KEY = "acb_usd_fx_rates_cache";

/** Reactive flag tracking whether a rates cache exists in localStorage. */
export const ratesCacheExists = ref(localStorage.getItem(STORAGE_KEY) !== null);

export function loadRatesCache(): RatesCacheData | undefined {
   try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (!raw) return undefined;
      return JSON.parse(raw) as RatesCacheData;
   } catch (e) {
      console.warn("Failed to load FX rates cache from localStorage:", e);
      return undefined;
   }
}

export function mergeRatesCacheUpdate(update: RatesCacheUpdate): void {
   if (update.years.length === 0) return;

   try {
      const existing = loadRatesCache() ?? { years: [] };
      const yearMap = new Map<number, RatesCacheUpdate["years"][number]>();

      for (const yr of existing.years) {
         yearMap.set(yr.year, yr);
      }
      // Overwrite with freshly-downloaded years
      for (const yr of update.years) {
         yearMap.set(yr.year, yr);
      }

      const merged: RatesCacheData = {
         years: Array.from(yearMap.values()).sort((a, b) => a.year - b.year),
      };
      localStorage.setItem(STORAGE_KEY, JSON.stringify(merged));
      ratesCacheExists.value = true;
   } catch (e) {
      console.warn("Failed to save FX rates cache to localStorage:", e);
   }
}

export function clearRatesCache(): void {
   localStorage.removeItem(STORAGE_KEY);
   ratesCacheExists.value = false;
}
