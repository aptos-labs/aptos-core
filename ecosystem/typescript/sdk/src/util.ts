import * as Gen from "./generated/index";

export type Nullable<T> = { [P in keyof T]: T[P] | null };

export type AnyObject = { [key: string]: any };

export async function sleep(timeMs: number): Promise<null> {
  return new Promise((resolve) => {
    setTimeout(resolve, timeMs);
  });
}

export function toMoveStructTagParam(moveStructTag: Gen.MoveStructTag): Gen.MoveStructTagParam {
  let genericTypeParamsString = "";
  if (moveStructTag.generic_type_params.length > 0) {
    genericTypeParamsString = `<${moveStructTag.generic_type_params.join(",")}>`;
  }
  return `${moveStructTag.address}::${moveStructTag.module}::${moveStructTag.name}${genericTypeParamsString}`;
}

// Note: This is not tested against generic type params that themselves have generic type params.
const moveStructTagParamRegex = /^(0x[0-9a-zA-Z_]+)::([0-9a-zA-Z_]+)::([0-9a-zA-Z_]+)(?:<([0-9a-zA-Z:_<>,]+)>){0,1}$/;

export function fromMoveStructTagParam(moveStructTagParam: Gen.MoveStructTagParam): Gen.MoveStructTag {
  const test = moveStructTagParamRegex.exec(moveStructTagParam);
  return {
    address: test[1],
    module: test[2],
    name: test[3],
    generic_type_params: test[4]?.split(",") ?? [],
  };
}
