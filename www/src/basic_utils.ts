/**
 * Gets the attribute @param key from @param obj.
 * If this attribute is not defined, then return the default @param def
 * The caller should generally expect the default to be the same type as the object attr.
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function getOr<D>(obj: any, key: string, def: D): D {
   if (obj === undefined) {
      return def;
   }
   // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access, @typescript-eslint/no-unsafe-assignment
   const value = obj[key];
   if (value === undefined) {
      return def;
   }
   if ((typeof value) === (typeof def)) {
      return value as D;
   }
   throw new TypeError(
      // eslint-disable-next-line @typescript-eslint/restrict-template-expressions
      `Expected ${key} to be of type ${typeof def}, but got ${typeof value}: ${value}`);
}

// eslint-disable-next-line @typescript-eslint/no-unnecessary-type-parameters, @typescript-eslint/no-explicit-any
export function get<T>(obj: any, key: string): T | null {
   // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access, @typescript-eslint/no-unsafe-assignment
   let val = obj[key];
   if (val === undefined) {
      return null;
   }
   return val as T;
}

// eslint-disable-next-line @typescript-eslint/no-unnecessary-type-parameters, @typescript-eslint/no-explicit-any
export function mustGet<T>(obj: any, key: string): T {
   // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access, @typescript-eslint/no-unsafe-assignment
   let val = obj[key];
   if (val === undefined) {
      // eslint-disable-next-line @typescript-eslint/restrict-template-expressions
       throw new TypeError(`No attribute ${key}: ${obj}`)
   }
   return val as T;
}

// Mainly to be used with Result type
export type UnitT = undefined;
export function Unit(): UnitT {
   return undefined;
}
