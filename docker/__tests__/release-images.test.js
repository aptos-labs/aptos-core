#!/usr/bin/env -S pnpm test release-images.test.js

import { getImageReleaseGroupByImageTagPrefix, } from '../release-images.mjs';
import { lazyImports, isReleaseImage, assertTagMatchesSourceVersion } from '../image-helpers.js';
describe('releaseImages', () => {
    it('gets velor-node as the default image group', () => {
        const prefix = 'image-banana';
        const releaseGroup = getImageReleaseGroupByImageTagPrefix(prefix);
        expect(releaseGroup).toEqual('velor-node');
    });
    it('gets indexer image group', () => {
        const prefix = 'velor-indexer-grpc-vX.Y.Z';
        const releaseGroup = getImageReleaseGroupByImageTagPrefix(prefix);
        expect(releaseGroup).toEqual('velor-indexer-grpc');
    });
    it('gets velor-node as the node image group', () => {
        const prefix = 'velor-node-vX.Y.Z';
        const releaseGroup = getImageReleaseGroupByImageTagPrefix(prefix);
        expect(releaseGroup).toEqual('velor-node');
    });
    it('determines image is a release image', () => {
        expect(isReleaseImage("nightly-banana")).toEqual(false);
        expect(isReleaseImage("velor-node-v1.2.3")).toEqual(true);
    });
    it('asserts version match', async () => {
        await lazyImports();
        // toThrow apparently matches a prefix, so this works but it does actually test against the real config version
        // Which... hilariously means this would fail if the version was ever 0.0.0
        expect(() => assertTagMatchesSourceVersion("velor-node-v0.0.0")).toThrow("image tag does not match cargo version: velor-node-v0.0.0");
    });
});
