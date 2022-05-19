import express from 'express';
import { PrismaClient } from '@prisma/client';

const prisma = new PrismaClient();
const app = express();

app.use(express.json());

app.get('/get-transaction-hashes', async (req, res) => {
  const transactions = await prisma.transactions.findMany();
  const hashes = transactions.map((transaction) => transaction.hash);
  res.json(hashes);
});

// eslint-disable-next-line @typescript-eslint/no-unused-vars
const server = app.listen(3000, () => (
  // eslint-disable-next-line no-console
  console.log(`
🚀 Server ready at: http://localhost:3000
⭐️ See sample requests: http://pris.ly/e/ts/rest-express#3-using-the-rest-api`)
));
