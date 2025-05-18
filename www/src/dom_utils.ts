export function childHasFocus(elem: HTMLElement) {
   for (const child of elem.children) {
      if (child === document.activeElement) {
         return true;
      }
   }
   return false;
}

class ElemBuilderT<T extends HTMLElement,
                   B extends ElemBuilderT<T, B>> {
   protected e: T;

   constructor(type: string) {
      this.e = document.createElement(type) as T;
   }

   public build(): T {
      return this.e;
   }

   public classes(classes_: Array<string>): B {
      for (const clz of classes_) {
         this.e.classList.add(clz);
      }
      return this as unknown as B;
   }

   public children(children_: Array<HTMLElement>): B {
      for (const child of children_) {
         this.e.appendChild(child);
      }
      return this as unknown as B;
   }

   public eventListener(
         eventName: string,
         handler: EventListenerOrEventListenerObject): B {
      this.e.addEventListener(eventName, handler);
      return this as unknown as B;
   }

   public text(t: string): B {
      this.e.innerText = t;
      return this as unknown as B;
   }
}

export class ElemBuilder extends ElemBuilderT<HTMLElement, ElemBuilder>{}

class InputElemBuilderT<
   T extends HTMLInputElement,
   B extends InputElemBuilderT<T, B>> extends ElemBuilderT<T, B> {

   constructor(type: string) {
      super("input");
      this.e.type = type;
   }

   public placeholder(placeholder_: string): B {
      this.e.placeholder = placeholder_;
      return this as unknown as B;
   }

   public pattern(pattern_: string): B {
      this.e.pattern = pattern_;
      return this as unknown as B;
   }
}

export class InputElemBuilder
   extends InputElemBuilderT<HTMLInputElement, InputElemBuilder>{}
