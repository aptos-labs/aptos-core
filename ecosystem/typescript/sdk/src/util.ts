import * as Gen from "./generated/index";

export type Nullable<T> = { [P in keyof T]: T[P] | null };

export type AnyObject = { [key: string]: any };

export async function sleep(timeMs: number): Promise<null> {
  return new Promise((resolve) => {
    setTimeout(resolve, timeMs);
  });
}

export function moveStructTagToParam(moveStructTag: Gen.MoveStructTag): Gen.MoveStructTagParam {
  let genericTypeParamsString = "";
  if (moveStructTag.generic_type_params.length > 0) {
    genericTypeParamsString = `<${moveStructTag.generic_type_params.join(",")}>`;
  }
  return `${moveStructTag.address}::${moveStructTag.module}::${moveStructTag.name}${genericTypeParamsString}`;
}
