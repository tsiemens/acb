import { ElementModel } from "./model_lib.js";

export class CollapsibleRegion extends ElementModel {
    public static readonly BUTTON_CLASS: string = "collapsible-content-btn";

    public static initAll() {
        const collapsibles = document.querySelectorAll(`.${CollapsibleRegion.BUTTON_CLASS}`);

        collapsibles.forEach(button => {
            button.addEventListener('click', (event) => {
                if (!event.target) {
                    return;
                }
                CollapsibleRegion.doToggle(event);
            });
        });
    }

    public static doToggle(buttonEvent: Event) {
        if (!buttonEvent.target) {
            return;
        }
        const button = (buttonEvent.target as Element)
            .closest(`.${CollapsibleRegion.BUTTON_CLASS}`) as HTMLElement;
        const content = button.nextElementSibling as HTMLElement | null;

        button.classList.toggle('active');

        const COLLAPSED = 'collapsed';
        const EXPANDED = 'expanded';

        if (content) {
            if (content.classList.contains(COLLAPSED)) {
                content.classList.remove(COLLAPSED);
                content.classList.add(EXPANDED);
                button.classList.add(EXPANDED);
            } else {
                content.classList.remove(EXPANDED);
                content.classList.add(COLLAPSED);
                button.classList.remove(EXPANDED);
            }
        }
    }
}