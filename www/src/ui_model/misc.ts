import { ElementModel } from "./model_lib.js";
import { webappVersion } from "../versions.js";

export class WasmVersionDisplay extends ElementModel {
    public static readonly ID: string = "acbVersion";

    public static get(): WasmVersionDisplay {
        return new WasmVersionDisplay(
            ElementModel.getRequiredElementById(WasmVersionDisplay.ID));
    }

    public setVersion(version: string) {
        this.element.innerText = `acb v${version}, acb-web v${webappVersion}`;
    }
}