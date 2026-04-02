import { JSONValue, loadJSON, loadText } from './http_utils.js';
import { ErrorBox } from './vue/error_box_store.js';
import { getSidebarInfoStore } from './vue/sidebar_info_store.js';

// Exported only for debugging
export function handleGitUserCaveatIssues(jsonObj: JSONValue) {
   console.log(jsonObj);
   const store = getSidebarInfoStore();
   // This json object will be a list when successful, and a dict with a message
   // on error.
   if (jsonObj instanceof Array) {
      console.log("json obj length:", jsonObj.length);
      ErrorBox.getGitIssues().hide();
      store.gitIssuesVisible = jsonObj.length > 0;
      if (jsonObj.length > 0) {
         store.warningsSectionVisible = true;
      }
   } else if ((jsonObj instanceof Object) && 'message' in jsonObj) {
      console.log("loadGitUserCaveatIssues failed!");
      console.log(jsonObj);
      ErrorBox.getGitIssues().showWith({
         title: "Open Issues Load Error",
         errorText: jsonObj.message as string,
      });
      store.warningsSectionVisible = true;
   } else {
      ErrorBox.getGitIssues().showWith({
         title: "Open Issues Load Error",
         descPre: "Received unknown format",
      });
      store.warningsSectionVisible = true;
   }
}

export function loadGitUserCaveatIssues() {
   const url = "https://api.github.com/repos/tsiemens/acb/issues?state=open&labels=user%20caveat"

   loadJSON(url).then((jsonObj) => {
      handleGitUserCaveatIssues(jsonObj);
   }).catch((error: unknown) => {
      console.log("loadGitUserCaveatIssues error:", error);
      ErrorBox.getGitIssues().showWith({
         title: "Open Issues Load Error",
         // eslint-disable-next-line @typescript-eslint/restrict-template-expressions
         errorText: `${error}`,
      });
      getSidebarInfoStore().warningsSectionVisible = true;
   });
}

const RAW_GITHUB_BASE = "https://raw.githubusercontent.com/tsiemens/acb/master";

function parseAcbVersionFromAppRs(text: string): string | null {
   const match = /ACB_APP_VERSION:\s*&str\s*=\s*"([^"]+)"/.exec(text);
   return match ? match[1] : null;
}

function parseWebappVersionFromVersionsTs(text: string): string | null {
   const match = /webappVersion\s*=\s*"([^"]+)"/.exec(text);
   return match ? match[1] : null;
}

// Exported for testing
export function checkVersionMismatch(
   localAcbVersion: string,
   localWebappVersion: string,
   remoteAcbVersion: string,
   remoteWebappVersion: string,
): void {
   const mismatches: string[] = [];
   if (localAcbVersion !== remoteAcbVersion) {
      mismatches.push(`acb: local v${localAcbVersion} vs latest v${remoteAcbVersion}`);
   }
   if (localWebappVersion !== remoteWebappVersion) {
      mismatches.push(`acb-web: local v${localWebappVersion} vs latest v${remoteWebappVersion}`);
   }

   if (mismatches.length > 0) {
      console.log("Version mismatch detected:", mismatches);
      const store = getSidebarInfoStore();
      ErrorBox.getVersionWarning().showWith({
         title: "New Version Available",
         descPre: "Your page may be using a cached older version.\n" +
            mismatches.join("\n"),
         descPost: "Try a hard refresh (Ctrl+Shift+R / Ctrl+F5) to update.",
      });
      store.warningsSectionVisible = true;
   }
}

export function loadAndCheckVersions(
   localAcbVersion: string,
   localWebappVersion: string,
): void {
   const acbVersionUrl = `${RAW_GITHUB_BASE}/src/app.rs`;
   const webappVersionUrl = `${RAW_GITHUB_BASE}/www/src/versions.ts`;

   Promise.all([
      loadText(acbVersionUrl),
      loadText(webappVersionUrl),
   ]).then(([appRsText, versionsTsText]) => {
      const remoteAcbVersion = parseAcbVersionFromAppRs(appRsText);
      const remoteWebappVersion = parseWebappVersionFromVersionsTs(versionsTsText);

      if (!remoteAcbVersion || !remoteWebappVersion) {
         console.warn("Could not parse remote versions",
            { remoteAcbVersion, remoteWebappVersion });
         return;
      }

      checkVersionMismatch(
         localAcbVersion, localWebappVersion,
         remoteAcbVersion, remoteWebappVersion,
      );
   }).catch((error: unknown) => {
      console.warn("Failed to check for version updates:", error);
      // Silently fail — this is a non-critical check
   });
}
