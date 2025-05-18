import { ElementModel } from "./model_lib.js";

export class GitIssuesInfo extends ElementModel {
    public static readonly ID: string = "git-issues-info";

    public static get(): GitIssuesInfo {
        return new GitIssuesInfo(
            ElementModel.getRequiredElementById(GitIssuesInfo.ID));
    }
}

export class GitIssuesLoadErrorP extends ElementModel {
    public static readonly ID: string = "git-issues-load-error";

    public static get(): GitIssuesLoadErrorP {
        return new GitIssuesLoadErrorP(
            ElementModel.getRequiredElementById(GitIssuesLoadErrorP.ID));
    }

    public setError(error: string | null) {
        if (error) {
            this.element.innerText = "Error loading git caveat issues: " + error;
            this.show();
         } else {
            this.hide()
         }
    }
}