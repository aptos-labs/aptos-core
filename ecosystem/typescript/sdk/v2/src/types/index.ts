export type AnyNumber = bigint | number;
export type AccountData = {
  sequence_number: string;
  authentication_key: string;
};
export interface PaginationArgs {
  start?: AnyNumber;
  limit?: number;
}
