type ValidObject = object | string | number | boolean;

// A typescript generic which emulates the Result type in Rust
export class Result<T extends ValidObject, E extends ValidObject> {
   private constructor(
      private value?: T | undefined,
      private error?: E | undefined,
   ) {}

   public static Ok<T extends ValidObject,
                    E extends ValidObject>(value: T): Result<T, E> {
      return new Result<T, E>(value, undefined);
   }

   public static Err<T extends ValidObject,
                     E extends ValidObject>(error: E): Result<T, E> {
      return new Result<T, E>(undefined, error);
   }

   public isOk(): boolean {
      return this.value !== undefined;
   }

   public isErr(): boolean {
      return this.error !== undefined;
   }

   public unwrap(): T {
      if (this.isOk()) {
         return this.value as T;
      }
      if (this.error === undefined) {
         throw new Error("Error unexpectedly undefined");
      }
      // eslint-disable-next-line @typescript-eslint/no-base-to-string
      throw new Error(`Tried to unwrap an error: ${this.error.toString()}`);
   }

   public unwrapErr(): E {
      if (this.isErr()) {
         return this.error as E;
      }
      if (this.value === undefined) {
         throw new Error("Value unexpectedly undefined");
      }
      // eslint-disable-next-line @typescript-eslint/no-base-to-string
      throw new Error(`Tried to unwrap an ok: ${this.value.toString()}`);
   }

   public match<U>(
      okFn: (value: T) => U,
      errFn: (error: E) => U,
   ): U {
      if (this.isOk()) {
         return okFn(this.value as T);
      } else {
         return errFn(this.error as E);
      }
   }
}