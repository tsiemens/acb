import { ElementModel } from "./model_lib.js";

export class WasmVersionDisplay extends ElementModel {
    public static readonly ID: string = "acb-version";

    public static get(): WasmVersionDisplay {
        return new WasmVersionDisplay(
            ElementModel.getRequiredElementById(WasmVersionDisplay.ID));
    }

    public setVersion(version: string) {
        this.element.innerText = "ACB Version: " + version;
    }
}