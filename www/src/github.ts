import { loadJSON } from './http_utils.js';
import { GitIssuesInfo, GitIssuesLoadErrorP } from './ui_model/github.js';

export function loadGitUserCaveatIssues() {
   const url = "https://api.github.com/repos/tsiemens/acb/issues?state=open&labels=user%20caveat"

   loadJSON(url).then((jsonObj) => {
      console.log(jsonObj);
      // This json object will be a list when successful, and a dict with a message
      // on error.
      if (jsonObj instanceof Array) {
         console.log("json obj length:", jsonObj.length);
         GitIssuesLoadErrorP.get().setError(null);
         GitIssuesInfo.get().element.hidden = (jsonObj.length == 0);
      } else if ((jsonObj instanceof Object) && 'message' in jsonObj) {
         console.log("loadGitUserCaveatIssues failed!");
         console.log(jsonObj);
         GitIssuesLoadErrorP.get().setError((jsonObj.message as string).toString());
      } else {
         GitIssuesLoadErrorP.get().setError("Received unknown format");
      }
   }).catch((error: unknown) => {
      console.log("loadGitUserCaveatIssues error:", error);
         GitIssuesLoadErrorP.get().setError("Unknown");
   });
 }