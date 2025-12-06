
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

    public getRequiredSubElementByClass(classname: string): HTMLElement {
        let elems = this.element.getElementsByClassName(classname);
        if (elems.length === 0) {
            throw Error(`Could not find sub Element .${classname}`);
        }
        return elems[0] as HTMLElement;
    }

    /** @virtual */
    public setHidden(hidden: boolean): void {
        this.element.hidden = hidden;
    }
    /** @virtual */
    public isHidden(): boolean {
        return this.element.hidden;
    }
    public hide() {
        this.setHidden(true);
    }
    public show() {
        this.setHidden(false);
    }
    public toggleHidden() {
        this.setHidden(!this.isHidden());
    }

    public setText(text: string) {
        this.element.innerText = text;
    }
}

export class ButtonElementModel extends ElementModel {
    public setEnabled(enabled: boolean) {
        // Valid for any btn/btn-primary/btn-secondary
        if (enabled) {
           this.element.removeAttribute("disabled");
        } else {
           this.element.setAttribute("disabled", "true");
        }
     }

    public setClickListener(callback: (_event: MouseEvent) => void) {
        this.element.addEventListener('click', callback);
    }
}

export class CheckboxElementModel extends ElementModel {
    public isChecked(): boolean {
        return (this.element as HTMLInputElement).checked;
    }

    public setChecked(checked: boolean) {
        (this.element as HTMLInputElement).checked = checked;
    }

    public setChangeListener(callback: (_event: Event) => void) {
        this.element.addEventListener('change', callback);
    }
}