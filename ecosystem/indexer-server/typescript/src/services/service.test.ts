/* eslint-disable @typescript-eslint/naming-convention */
/* eslint-disable @typescript-eslint/no-unused-vars */
import axios from 'axios';
import prisma from './Prisma';

test('/tokens/all', async () => {
  const token_id = new Date().toString();
  const token = await prisma.tokens.create({
    data: {
      collection: 'Lazer Ape',
      creator: 'Aptos',
      description: 'A collection of apes',
      max_amount: 1,
      minted_at: new Date(),
      name: 'Ape',
      supply: 1,
      token_id,
      uri: 'https://aptoslabs.com',
    },
  });

  await prisma.tokens.delete({
    where: {
      token_id,
    },
  });
  // const fetchedToken = await (await (await axios.get('http://localhost:4000/tokens/all')).data).json();
  // expect(fetchedToken).toBe(token);
});
