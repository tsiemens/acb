import { JSONValue, loadJSON } from './http_utils.js';
import { ErrorBox, SidebarWarningsSection } from './ui_model/error_displays.js';
import { GitIssuesInfo } from './ui_model/github.js';

// Exported only for debugging
export function handleGitUserCaveatIssues(jsonObj: JSONValue) {
   console.log(jsonObj);
   // This json object will be a list when successful, and a dict with a message
   // on error.
   if (jsonObj instanceof Array) {
      console.log("json obj length:", jsonObj.length);
      ErrorBox.getGitIssues().hide();
      GitIssuesInfo.get().setHidden(jsonObj.length == 0);
   } else if ((jsonObj instanceof Object) && 'message' in jsonObj) {
      console.log("loadGitUserCaveatIssues failed!");
      console.log(jsonObj);
      ErrorBox.getGitIssues().showWith({
         title: "Open Issues Load Error",
         errorText: (jsonObj.message as string).toString(),
      });
      SidebarWarningsSection.get().show();
   } else {
      ErrorBox.getGitIssues().showWith({
         title: "Open Issues Load Error",
         descPre: "Received unknown format",
      });
      SidebarWarningsSection.get().show();
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
      SidebarWarningsSection.get().show();
   });
 }