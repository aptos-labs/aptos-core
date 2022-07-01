# Aptos Wallet Website
## Getting Started

### Development


First, run the development server:

```bash
npm run dev
# or
yarn dev
```


### Production

When creating a production build use:

```bash
yarn run lint
```

```bash
yarn run build
```

To view the production build locally, run:

```bash
yarn run start
```

## Documentation
Docs are located in `website/docs` and are denoted with a `.mdx` file extension. When re-ordering or creating new docs, ensure that the `docsSlugOrdering` in `mdxUtils.ts` is also updated to reflect the changes. Also make sure to change `next.config.js` redirects if the `/docs` default redirect changes.

### API

[API routes](https://nextjs.org/docs/api-routes/introduction) can be accessed on [http://localhost:3000/api/hello](http://localhost:3000/api/hello). This endpoint can be edited in `pages/api/hello.ts`.

The `pages/api` directory is mapped to `/api/*`. Files in this directory are treated as [API routes](https://nextjs.org/docs/api-routes/introduction) instead of React pages.

##