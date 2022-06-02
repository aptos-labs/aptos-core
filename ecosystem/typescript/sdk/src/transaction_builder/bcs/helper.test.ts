import { AccountAddress } from '../aptos_types';
import { Deserializer } from './deserializer';
import { bcsToBytes, deserializeVector, serializeVector } from './helper';
import { Serializer } from './serializer';

test('serializes and deserializes a vector of serializables', () => {
  const address0 = AccountAddress.fromHex('0x1');
  const address1 = AccountAddress.fromHex('0x2');

  const serializer = new Serializer();
  serializeVector([address0, address1], serializer);

  const addresses: AccountAddress[] = deserializeVector(new Deserializer(serializer.getBytes()), AccountAddress);

  expect(addresses[0].address).toEqual(address0.address);
  expect(addresses[1].address).toEqual(address1.address);
});

test('bcsToBytes', () => {
  const address = AccountAddress.fromHex('0x1');
  bcsToBytes(address);

  expect(bcsToBytes(address)).toEqual(
    new Uint8Array([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]),
  );
});
