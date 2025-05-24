import { ElementModel } from "./model_lib.js";

export class CollapsibleRegion extends ElementModel {
    public static readonly BUTTON_CLASS: string = "collapsible-region-btn";
    public static readonly CONTENT_CLASS: string = "collapsible-region-content";

    public static initAll() {
        const collapsibles = document.querySelectorAll(`.${CollapsibleRegion.BUTTON_CLASS}`);

        collapsibles.forEach(button => {
            button.addEventListener('click', (event) => {
                if (!event.target) {
                    return;
                }
                let button = (event.target as Element)
                    .closest(`.${CollapsibleRegion.BUTTON_CLASS}`) as HTMLElement;
                button.classList.toggle('active');
                const content = button.nextElementSibling as HTMLElement | null;
                if (content) {
                    if (content.style.maxHeight) {
                        content.style.maxHeight = '';
                    } else {
                        content.style.maxHeight = content.scrollHeight.toString() + 'px';
                    }
                }
            });
        });
    }
}