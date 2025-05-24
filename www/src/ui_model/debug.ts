import { handleGitUserCaveatIssues } from "../github.js";
import { ElementModel, ButtonElementModel } from "./model_lib.js";

// This is in debug for now, since the only thing in it is the debug button
// It is hidden by default, and only shown when something in it is enabled.
class UserActionsGroup extends ElementModel {
   public static readonly CLASS: string = "user-actions";

   public static get(): UserActionsGroup {
      return new UserActionsGroup(
        ElementModel.getRequiredElementByQuery(`.${UserActionsGroup.CLASS}`));
   }

   /** @override */
   public setHidden(hidden: boolean): void {
      this.element.style.display = (hidden ? 'none' : 'block');
  }
  /** @override */
  public isHidden(): boolean {
      return this.element.style.display === 'none';
  }
}

function isDebugModeEnabled(): boolean {
   // Check if the hostname is localhost or some raw IP address.
   const hostIsDebug = window.location.hostname === "localhost" ||
      window.location.hostname.match(/^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$/);

   // We can disable debug mode in the URL (mainly just for checking header layout)
   const debugModeDisabled = window.location.search.includes("debug=false");

   return (hostIsDebug && !debugModeDisabled) ? true : false;
}

export class DebugSettingsButton extends ButtonElementModel {
   public static readonly ID: string = "debugSettingsButton";

   public static get(): DebugSettingsButton {
      return new DebugSettingsButton(
         ElementModel.getRequiredElementById(DebugSettingsButton.ID));
   }

   public setup() {
      this.setClickListener((_event) => {
         new ElementModel(ElementModel.getRequiredElementById("debugSettingsPanel"))
            .toggleHidden();
      });

      if (isDebugModeEnabled()) {
         console.debug("Debug mode enabled");
         UserActionsGroup.get().show();
      }
   }
}

class GithubOpenIssuesButton extends ButtonElementModel {
   public static readonly ID: string = "generateGithubOpenIssuesButton";

   public static get(): GithubOpenIssuesButton {
      return new GithubOpenIssuesButton(
         ElementModel.getRequiredElementById(GithubOpenIssuesButton.ID));
   }

   public setup() {
      this.setClickListener((_event) => {
         handleGitUserCaveatIssues(["some caveat"]);
      });
   }
}

class GithubFetchErrorButton extends ButtonElementModel {
   public static readonly ID: string = "generateGithubFetchErrorButton";

   public static get(): GithubFetchErrorButton {
      return new GithubFetchErrorButton(
         ElementModel.getRequiredElementById(GithubFetchErrorButton.ID));
   }

   public setup() {
      this.setClickListener((_event) => {
        const date = new Date();
         if ((date.getSeconds() % 2) == 0) {
            handleGitUserCaveatIssues({"message": "Sample issue from github"});
         } else {
            // Invalid format
            handleGitUserCaveatIssues({"bla": "bar"});
         }
      });
   }
}

// When this checkbox is clicked, we'll re-load the page with debug_autoload=true
// in the URL, which will trigger autoload when the page loads.
export class AutoRunCheckbox extends ElementModel {
   public static readonly ID: string = "debugAutoloadCheckbox";

   public static get(): AutoRunCheckbox {
      return new AutoRunCheckbox(
         ElementModel.getRequiredElementById(AutoRunCheckbox.ID));
   }

   public checked(): boolean {
      return (this.element as HTMLInputElement).checked;
   }

   public setup() {
      const checked = isDebugModeEnabled() &&
         window.location.search.includes("debug_autoload=true");
      (this.element as HTMLInputElement).checked = checked;

      this.element.addEventListener('change', () => {
         // If checked, add debug_autoload=true to the URL, else
         // remove it. In either case, reload the page with that URL.
         const url = new URL(window.location.href);
         if (AutoRunCheckbox.get().checked()) {
            url.searchParams.set("debug_autoload", "true");
         } else {
            url.searchParams.delete("debug_autoload");
         }
         // Reload page
         window.location.href = url.toString();
      })
   }
}

export class DebugSettings {
   public static init() {
      DebugSettingsButton.get().setup();

      // Inside debug panel
      GithubFetchErrorButton.get().setup();
      GithubOpenIssuesButton.get().setup();
      AutoRunCheckbox.get().setup();
   }
}
