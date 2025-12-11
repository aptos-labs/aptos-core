import * as core from '@actions/core';
import * as exec from '@actions/exec';
import { DefaultArtifactClient } from '@actions/artifact';
import * as fs from 'fs';
import * as path from 'path';
import * as toml from '@iarna/toml';

interface BuildConfig {
  buildDefaults: boolean;
  binaries: string[];
  profile: string;
  separateArtifacts: boolean;
  gitSha: string;
}

interface BinaryInfo {
  name: string;
  path: string;
  size: number;
}

interface CargoToml {
  workspace?: {
    'default-members'?: string[];
  };
}

async function run(): Promise<void> {
  try {
    // Parse inputs
    const config = parseInputs();
    
    core.info('🔨 Build Configuration:');
    core.info(`  Profile: ${config.profile}`);
    core.info(`  Build defaults: ${config.buildDefaults}`);
    core.info(`  Additional binaries: ${config.binaries.join(', ') || 'none'}`);
    core.info(`  Separate artifacts: ${config.separateArtifacts}`);
    core.info(`  Git SHA: ${config.gitSha}`);
    core.info('');

    // Build binaries (assumes Nix is already installed by workflow)
    await buildBinaries(config);

    // Determine output directory
    const targetFolder = getTargetFolder(config.profile);
    core.info(`📁 Target folder: ${targetFolder}`);

    // Discover and verify built binaries
    const builtBinaries = await discoverBuiltBinaries(config, targetFolder);

    // Upload artifacts
    const shortSha = config.gitSha.substring(0, 7);
    
    if (config.separateArtifacts) {
      await uploadSeparateArtifacts(builtBinaries, shortSha, targetFolder);
      core.setOutput('artifact_name', `{binary}-${shortSha}`);
    } else {
      const artifactName = `all-binaries-${shortSha}`;
      await uploadCombinedArtifact(builtBinaries, artifactName, targetFolder);
      core.setOutput('artifact_name', artifactName);
    }

    // Set outputs
    core.setOutput('binaries_built', JSON.stringify(builtBinaries.map(b => b.name)));

    core.info('✅ Build completed successfully!');
  } catch (error) {
    if (error instanceof Error) {
      core.setFailed(error.message);
    } else {
      core.setFailed('An unknown error occurred');
    }
  }
}

function parseInputs(): BuildConfig {
  const buildDefaults = core.getInput('defaults') === 'true';
  const separateArtifacts = core.getInput('separate_artifacts') === 'true';
  const binariesInput = core.getInput('binaries');
  
  let binaries: string[] = [];
  if (binariesInput) {
    try {
      const parsed = JSON.parse(binariesInput);
      if (Array.isArray(parsed)) {
        binaries = parsed.map(b => String(b).trim()).filter(b => b.length > 0);
      } else {
        binaries = [String(parsed).trim()].filter(b => b.length > 0);
      }
    } catch {
      binaries = binariesInput
        .split(/[,\n]/)
        .map((b: string) => b.trim())
        .filter((b: string) => b.length > 0);
    }
  }
  
  const profile = core.getInput('profile', { required: true });
  
  // Get git_sha input, fall back to GITHUB_SHA if not provided
  const gitShaInput = core.getInput('git_sha');
  const gitSha = gitShaInput || process.env.GITHUB_SHA || 'unknown';

  return {
    buildDefaults,
    binaries,
    profile,
    separateArtifacts,
    gitSha,
  };
}

async function buildBinaries(config: BuildConfig): Promise<void> {
  const profileArg = config.profile === 'release' ? '--release' : `--profile ${config.profile}`;

  if (config.buildDefaults) {
    core.info('📦 Building all default-member binaries...');
    core.info('   (This builds all packages in default-members that produce binaries)');
    await exec.exec('nix', [
      '--extra-experimental-features',
      'nix-command flakes',
      'develop',
      '-c',
      'cargo',
      'build',
      ...profileArg.split(' '),
    ]);
    core.info('✅ Default-members build complete');
    core.info('');
  }

  if (config.binaries.length > 0) {
    core.info('📦 Building additional binaries...');
    for (const binary of config.binaries) {
      core.info(`  Building: ${binary}`);
      await exec.exec('nix', [
        '--extra-experimental-features',
        'nix-command flakes',
        'develop',
        '-c',
        'cargo',
        'build',
        ...profileArg.split(' '),
        '-p',
        binary,
      ]);
    }
    core.info('✅ Additional binaries build complete');
  }
}

function getTargetFolder(profile: string): string {
  if (profile === 'release') {
    return 'target/release';
  } else if (profile === 'dev') {
    return 'target/debug';
  } else {
    return `target/${profile}`;
  }
}

async function discoverBuiltBinaries(
  config: BuildConfig,
  targetFolder: string
): Promise<BinaryInfo[]> {
  core.info('📋 Discovering built binaries in target folder...');
  core.info('');

  if (!fs.existsSync(targetFolder)) {
    throw new Error(`Target folder not found: ${targetFolder}`);
  }

  const builtBinaries: BinaryInfo[] = [];
  const files = fs.readdirSync(targetFolder);

  // Filter for executable files (binaries)
  for (const file of files) {
    const filePath = path.join(targetFolder, file);
    const stats = fs.statSync(filePath);

    // Check if it's a file and executable
    if (stats.isFile() && (stats.mode & 0o111) !== 0) {
      // Skip files with extensions (like .d, .rlib, etc.)
      if (!file.includes('.')) {
        const sizeInMB = (stats.size / (1024 * 1024)).toFixed(2);
        core.info(`  ✅ ${file} (${sizeInMB} MB)`);
        builtBinaries.push({ name: file, path: filePath, size: stats.size });
      }
    }
  }

  core.info('');
  core.info(`📊 Total binaries found: ${builtBinaries.length}`);

  // Verify that additional binaries were built if specified
  if (config.binaries.length > 0) {
    const builtNames = builtBinaries.map(b => b.name);
    for (const binary of config.binaries) {
      if (!builtNames.includes(binary)) {
        throw new Error(`Expected binary not found: ${binary}`);
      }
    }
  }

  if (builtBinaries.length === 0) {
    throw new Error('No binaries were built!');
  }

  return builtBinaries;
}

async function uploadCombinedArtifact(
  binaries: BinaryInfo[],
  artifactName: string,
  targetFolder: string
): Promise<void> {
  core.info(`📤 Uploading ${binaries.length} binaries as single artifact: ${artifactName}`);

  const artifactClient = new DefaultArtifactClient();
  const files = binaries.map(b => b.path);
  const rootDirectory = path.resolve(targetFolder);

  const uploadResponse = await artifactClient.uploadArtifact(
    artifactName,
    files,
    rootDirectory,
    {
      retentionDays: 7,
    }
  );

  core.info(`✅ Artifact uploaded: ${artifactName}`);
  core.info(`   ID: ${uploadResponse.id}`);
  const sizeInMB = ((uploadResponse.size || 0) / (1024 * 1024)).toFixed(2);
  core.info(`   Size: ${sizeInMB} MB`);
}

async function uploadSeparateArtifacts(
  binaries: BinaryInfo[],
  shortSha: string,
  targetFolder: string
): Promise<void> {
  core.info(`📤 Uploading ${binaries.length} binaries as separate artifacts...`);
  core.info('');

  const artifactClient = new DefaultArtifactClient();
  const rootDirectory = path.resolve(targetFolder);

  for (const binary of binaries) {
    const artifactName = `${binary.name}-${shortSha}`;
    core.info(`  Uploading: ${artifactName}`);

    const uploadResponse = await artifactClient.uploadArtifact(
      artifactName,
      [binary.path],
      rootDirectory,
      {
        retentionDays: 7,
      }
    );

    const sizeInMB = ((uploadResponse.size || 0) / (1024 * 1024)).toFixed(2);
    core.info(`    ✅ ${artifactName} (${sizeInMB} MB, ID: ${uploadResponse.id})`);
  }

  core.info('');
  core.info(`✅ All ${binaries.length} artifacts uploaded successfully!`);
}

run();