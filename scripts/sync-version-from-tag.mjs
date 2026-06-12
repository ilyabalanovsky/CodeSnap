import { readFile, writeFile } from 'node:fs/promises';

const rawVersion = process.argv[2] || process.env.GITHUB_REF_NAME || process.env.npm_package_version;
const version = rawVersion?.trim().replace(/^refs\/tags\//, '').replace(/^v/i, '');

if (!version || !/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(version)) {
  throw new Error(`Expected a semver tag like v1.2.3, got "${rawVersion || ''}".`);
}

async function updateJson(path, updater) {
  const text = await readFile(path, 'utf8');
  const data = JSON.parse(text);
  updater(data);
  await writeFile(path, `${JSON.stringify(data, null, 2)}\n`);
}

function replaceFirst(text, pattern, replacement, path) {
  if (!pattern.test(text)) {
    throw new Error(`Could not find version field in ${path}.`);
  }

  return text.replace(pattern, replacement);
}

await updateJson('package.json', (data) => {
  data.version = version;
});

await updateJson('package-lock.json', (data) => {
  data.version = version;

  if (data.packages?.['']) {
    data.packages[''].version = version;
  }
});

await updateJson('src-tauri/tauri.conf.json', (data) => {
  data.version = version;
});

const cargoTomlPath = 'src-tauri/Cargo.toml';
const cargoToml = await readFile(cargoTomlPath, 'utf8');
await writeFile(
  cargoTomlPath,
  replaceFirst(cargoToml, /^version = ".*"$/m, `version = "${version}"`, cargoTomlPath),
);

const cargoLockPath = 'src-tauri/Cargo.lock';
const cargoLock = await readFile(cargoLockPath, 'utf8');
const cargoLockLines = cargoLock.split(/\r?\n/);
let inSnapScriptPackage = false;
let updatedCargoLock = false;

for (let index = 0; index < cargoLockLines.length; index += 1) {
  const line = cargoLockLines[index];

  if (line === '[[package]]') {
    inSnapScriptPackage = false;
    continue;
  }

  if (line === 'name = "snapscript"') {
    inSnapScriptPackage = true;
    continue;
  }

  if (inSnapScriptPackage && line.startsWith('version = ')) {
    cargoLockLines[index] = `version = "${version}"`;
    updatedCargoLock = true;
    break;
  }
}

if (!updatedCargoLock) {
  throw new Error(`Could not find SnapScript package entry in ${cargoLockPath}.`);
}

await writeFile(cargoLockPath, cargoLockLines.join('\n'));

console.log(`SnapScript release version synced to ${version}.`);
