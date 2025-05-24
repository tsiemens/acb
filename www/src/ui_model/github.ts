import { SidebarWarningsSection } from "./error_displays.js";
import { ElementModel } from "./model_lib.js";

// This just contains a static message in it.
export class GitIssuesInfo extends ElementModel {
    public static readonly ID: string = "gitIssuesInfo";

    public static get(): GitIssuesInfo {
        return new GitIssuesInfo(
            ElementModel.getRequiredElementById(GitIssuesInfo.ID));
    }

    /** @override */
    public setHidden(hidden: boolean): void {
        super.setHidden(hidden);
        if (!hidden) {
             SidebarWarningsSection.get().show();
        }
    }
}