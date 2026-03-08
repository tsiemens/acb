// TypeScript does not natively understand .vue files. This shim tells the TS
// compiler to treat any .vue import as a Vue component, suppressing type errors.
declare module '*.vue' {
   import type { DefineComponent } from 'vue';
   const component: DefineComponent;
   export default component;
}
