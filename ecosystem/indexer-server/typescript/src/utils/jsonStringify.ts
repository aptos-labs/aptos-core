export const jsonStringify = (key: any, value: any) => (
  typeof value === 'bigint'
    ? value.toString()
    : value // return everything else unchanged
);

export default jsonStringify;
