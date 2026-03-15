import { JSONValue, loadJSON } from './http_utils.js';
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
