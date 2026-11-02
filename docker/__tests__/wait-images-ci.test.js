import { CargoBuildFeatures, getImagesToWaitFor, joinTagSegments } from '../image-helpers.js';

function requiredTags(flags) {
    return Object.entries(getImagesToWaitFor(flags)).flatMap(([image, profiles]) =>
        Object.entries(profiles).flatMap(([profile, features]) =>
            features.map(feature => `${image}:${joinTagSegments(
                profile === 'release' ? '' : profile,
                feature === 'default' ? '' : feature,
                'test-sha',
            )}`),
        ),
    );
}

describe('CI image readiness', () => {
    it('cannot satisfy consensus readiness with ordinary main-branch images', () => {
        const available = new Set(requiredTags({}));
        const required = requiredTags({ FEATURE_CONSENSUS_ONLY: true });
        expect(required).toContain('validator-testing:consensus_only_perf_test_test-sha');
        expect(required).toContain('forge:consensus_only_perf_test_test-sha');
        expect(required.every(tag => available.has(tag))).toBe(false);
        required.forEach(tag => available.add(tag));
        expect(required.every(tag => available.has(tag))).toBe(true);
        expect(required.every(tag => tag.includes(':consensus_only_perf_test_'))).toBe(true);
    });

    it('preserves the default release, performance and failpoints checks', () => {
        const tags = requiredTags({});
        expect(tags).toEqual(requiredTags({
            PROFILE_RELEASE: true, PROFILE_PERF: true, FEATURE_FAILPOINTS: true,
        }));
        expect(tags).toContain('validator:test-sha');
        expect(tags).toContain('validator:performance_test-sha');
        expect(tags).toContain('validator:failpoints_test-sha');
        expect(tags.some(tag => tag.includes('consensus_only'))).toBe(false);
    });

    it('combines requested feature variants without dropping any', () => {
        const images = getImagesToWaitFor({
            PROFILE_RELEASE: true, FEATURE_FAILPOINTS: true, FEATURE_CONSENSUS_ONLY: true,
        });
        expect(images['validator-testing'].release).toEqual([
            CargoBuildFeatures.Default, CargoBuildFeatures.ConsensusOnly, CargoBuildFeatures.Failpoints,
        ]);
    });
});
