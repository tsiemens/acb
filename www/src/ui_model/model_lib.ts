
export class ElementModel {
    constructor(public element: HTMLElement) {}

    public static getRequiredElementById(id: string): HTMLElement {
        let elem = document.getElementById(id);
        if (elem === null) {
            throw Error(`Could not find Element #${id}`);
        } else {
            return elem;
        }
    }

    public static getRequiredElementByQuery(q: string): HTMLElement {
        let elem = document.querySelector(q);
        if (elem === null) {
            throw Error(`Could not find Element matching ${q}`);
        } else {
            return elem as HTMLElement;
        }
    }

    public hide() {
        this.element.hidden = true;
    }

    public show() {
        this.element.hidden = false;
    }

    public setText(text: string) {
        this.element.innerText = text;
    }
}