#!/usr/bin/env -S pnpm test release-images.test.js

import { getImageReleaseGroupByImageTagPrefix, } from '../release-images.mjs';
import { lazyImports, isReleaseImage, assertTagMatchesSourceVersion, retryWithBackoff } from '../image-helpers.js';
describe('releaseImages', () => {
    it('gets aptos-node as the default image group', () => {
        const prefix = 'image-banana';
        const releaseGroup = getImageReleaseGroupByImageTagPrefix(prefix);
        expect(releaseGroup).toEqual('aptos-node');
    });
    it('gets indexer image group', () => {
        const prefix = 'aptos-indexer-grpc-vX.Y.Z';
        const releaseGroup = getImageReleaseGroupByImageTagPrefix(prefix);
        expect(releaseGroup).toEqual('aptos-indexer-grpc');
    });
    it('gets aptos-node as the node image group', () => {
        const prefix = 'aptos-node-vX.Y.Z';
        const releaseGroup = getImageReleaseGroupByImageTagPrefix(prefix);
        expect(releaseGroup).toEqual('aptos-node');
    });
    it('determines image is a release image', () => {
        expect(isReleaseImage("nightly-banana")).toEqual(false);
        expect(isReleaseImage("aptos-node-v1.2.3")).toEqual(true);
    });
    it('asserts version match', async () => {
        await lazyImports();
        // toThrow apparently matches a prefix, so this works but it does actually test against the real config version
        // Which... hilariously means this would fail if the version was ever 0.0.0
        expect(() => assertTagMatchesSourceVersion("aptos-node-v0.0.0")).toThrow("image tag does not match cargo version: aptos-node-v0.0.0");
    });
});

describe('retryWithBackoff', () => {
    const noSleep = () => Promise.resolve();

    // the `jest` global isn't available in ESM test files, so hand-roll the fakes
    function failNTimes(failures, result = 'copied') {
        const fake = async () => {
            fake.calls++;
            if (fake.calls <= failures) {
                // this is the DockerHub 500 case that aborted the nightly release
                throw Object.assign(new Error('exit 1'), { stderr: 'UNKNOWN: 500 Internal Server Error' });
            }
            return result;
        };
        fake.calls = 0;
        return fake;
    }

    beforeAll(async () => {
        await lazyImports(); // for chalk
    });

    it('returns the result without retrying when the operation succeeds', async () => {
        const fn = failNTimes(0);
        expect(await retryWithBackoff('copy', fn, { sleepFn: noSleep })).toEqual('copied');
        expect(fn.calls).toEqual(1);
    });

    it('retries a transient registry failure and succeeds', async () => {
        const fn = failNTimes(2);
        expect(await retryWithBackoff('copy', fn, { sleepFn: noSleep })).toEqual('copied');
        expect(fn.calls).toEqual(3);
    });

    it('gives up after the configured number of attempts and rethrows', async () => {
        const fn = failNTimes(Infinity);
        await expect(retryWithBackoff('copy', fn, { attempts: 3, sleepFn: noSleep })).rejects.toThrow('exit 1');
        expect(fn.calls).toEqual(3);
    });

    it('backs off exponentially between attempts', async () => {
        const delays = [];
        await expect(retryWithBackoff('copy', failNTimes(Infinity), {
            attempts: 4,
            initialDelayMs: 5000,
            sleepFn: (ms) => { delays.push(ms); return Promise.resolve(); },
        })).rejects.toThrow('exit 1');
        expect(delays).toEqual([5000, 10000, 20000]);
    });
});
