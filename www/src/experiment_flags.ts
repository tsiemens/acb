export const TAMPERMONKEY_SCRIPT = "tampermonkey_script";

export const ALL_EXPERIMENT_FLAGS: string[] = [
   TAMPERMONKEY_SCRIPT,
];

export function isExperimentEnabled(flag: string): boolean {
   const params = new URLSearchParams(window.location.search);
   // Support ?experiment=flag1,flag2 or repeated ?experiment=flag1&experiment=flag2
   for (const val of params.getAll("experiment")) {
      if (val.split(",").map(s => s.trim()).includes(flag)) {
         return true;
      }
   }
   return false;
}

/** Returns the set of experiment flags currently enabled via the URL. */
export function getEnabledExperiments(): Set<string> {
   return new Set(ALL_EXPERIMENT_FLAGS.filter(isExperimentEnabled));
}

/**
 * Builds a URL with the given set of experiment flags applied.
 * Replaces any existing `experiment` params — other params are preserved.
 */
export function buildUrlWithExperiments(enabled: Set<string>): string {
   const url = new URL(window.location.href);
   url.searchParams.delete("experiment");
   for (const flag of enabled) {
      url.searchParams.append("experiment", flag);
   }
   return url.toString();
}
