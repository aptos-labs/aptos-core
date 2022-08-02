import { serialize } from 'next-mdx-remote/serialize';
import { MDXRemote } from 'next-mdx-remote';
import DocsLayout from 'core/layout/DocsLayout';
import fs from 'fs';
import path from 'path';
import { docsFilePaths, docsSlugOrdering, DOCS_PATH } from 'docs/mdxUtils';
import matter from 'gray-matter';
import {
  BASE_URL,
  COMPANY_NAME, COMPANY_URL, DEFAULT_SEO_DESCRIPTION,
} from 'core/constants';
import { NextSeo } from 'next-seo';

const image = 'Petra_Docs_Card.png';
const imageUrl = `${BASE_URL}/${image}` as const;

interface DocsGetStaticPropsProps {
  params: {
    slug: string;
  }
}

export const getStaticProps = async ({ params }: DocsGetStaticPropsProps) => {
  const docsRelativePath = `/docs/${params.slug}`;
  const docsPathJoin = path.join(DOCS_PATH, `${params.slug}.mdx`);
  const source = fs.readFileSync(docsPathJoin);

  const { content, data } = matter(source);

  const mdxSource = await serialize(content, {
    // Optionally pass remark/rehype plugins
    mdxOptions: {
      rehypePlugins: [],
      remarkPlugins: [],
    },
    scope: data,
  });

  let paths = docsFilePaths
    // Remove file extensions for page paths
    .map((value) => value.replace(/\.mdx?$/, ''))
    // Map the path into the static paths object required by Next.js
    .map((slug) => {
      const slugDocsPathJoin = path.join(DOCS_PATH, `${slug}.mdx`);
      const slugSource = fs.readFileSync(slugDocsPathJoin);
      const {
        data: pageData,
      } = matter(slugSource);

      const docsSlugOrderingIndex = docsSlugOrdering.indexOf(slug);
      const nextPage = (docsSlugOrderingIndex + 1 < docsSlugOrdering.length)
        ? docsSlugOrdering[docsSlugOrderingIndex + 1]
        : null;
      const prevPage = (docsSlugOrderingIndex - 1 >= 0)
        ? docsSlugOrdering[docsSlugOrderingIndex - 1]
        : null;

      pageData.nextPageSlug = nextPage;
      pageData.nextPagePath = (nextPage) ? `/docs/${nextPage}` : null;
      pageData.prevPageSlug = prevPage;
      pageData.prevPagePath = (prevPage) ? `/docs/${prevPage}` : null;

      return (
        {
          data: pageData,
          index: docsSlugOrderingIndex,
          path: `/docs/${slug}`,
          slug,
        }
      );
    });

  paths.sort((path1, path2) => path1.index - path2.index);

  paths = paths.map((tempPath, index) => {
    const nextPageTitle = (index + 1 < docsSlugOrdering.length)
      ? paths[index + 1].data.title
      : null;
    const prevPageTitle = (index - 1 >= 0)
      ? paths[index - 1].data.title
      : null;

    const tempData: Record<string, any> = { ...tempPath.data };

    tempData.nextPageTitle = nextPageTitle;
    tempData.prevPageTitle = prevPageTitle;
    return {
      ...tempPath,
      data: tempData,
    };
  });

  return {
    props: {
      docsPath: docsRelativePath,
      frontMatter: data,
      paths,
      source: mdxSource,
    },
  };
};

export type DocsGetStaticPropsReturn = Awaited<ReturnType<typeof getStaticProps>>;

export const getStaticPaths = async () => {
  const paths = docsFilePaths
    // Remove file extensions for page paths
    .map((value) => value.replace(/\.mdx?$/, ''))
    // Map the path into the static paths object required by Next.js
    .map((slug) => (
      { params: { slug } }
    ));

  return {
    fallback: false,
    paths,
  };
};

export default function DocsPage({
  docsPath, frontMatter, paths, source,
}: DocsGetStaticPropsReturn['props']) {
  const docTitle: string | undefined = frontMatter?.title;
  return (
    <DocsLayout paths={paths} docsPath={docsPath}>
      <NextSeo
        title={`${docTitle || 'Docs'} - ${COMPANY_NAME}`}
        description={DEFAULT_SEO_DESCRIPTION}
        openGraph={{
          description: DEFAULT_SEO_DESCRIPTION,
          images: [
            {
              alt: COMPANY_NAME,
              height: 600,
              type: 'image/jpeg',
              url: imageUrl,
              width: 800,
            },
          ],
          site_name: COMPANY_NAME,
          title: COMPANY_NAME,
          url: COMPANY_URL,
        }}
      />
      <MDXRemote {...source} />
    </DocsLayout>
  );
}
