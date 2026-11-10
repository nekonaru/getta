#!/usr/bin/env node
// Build latest.json for the Tauri updater from the matrix build artifacts.
// Fails loudly when an expected updater artifact is missing so a broken
// release can never publish a manifest that skips a platform.

const fs = require('fs');
const path = require('path');

const artifactsRoot = process.argv[2];
const tag = process.env.GITHUB_REF_NAME;
if (!artifactsRoot || !tag) {
	console.error('usage: GITHUB_REF_NAME=v1.2.3 node build-latest-json.js <artifacts-dir>');
	process.exit(1);
}
const version = tag.replace(/^v/, '');

function walk(dir) {
	const out = [];
	for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
		const full = path.join(dir, entry.name);
		if (entry.isDirectory()) out.push(...walk(full));
		else out.push(full);
	}
	return out;
}

const all = walk(artifactsRoot);
const windowsSig = all.find((f) => f.endsWith('.exe.sig'));
const linuxSig = all.find((f) => f.endsWith('.AppImage.sig'));
const macSigs = all.filter((f) => f.endsWith('.app.tar.gz.sig'));

if (!windowsSig) {
	console.error('missing windows updater signature (.exe.sig)');
	process.exit(1);
}
if (!linuxSig) {
	console.error('missing linux updater signature (.AppImage.sig)');
	process.exit(1);
}
if (macSigs.length !== 2) {
	console.error(`expected 2 mac updater signatures (.app.tar.gz.sig), found ${macSigs.length}`);
	process.exit(1);
}

const sigOf = (path) => fs.readFileSync(path, 'utf8').trim();
const urlFor = (assetPath) =>
	`https://github.com/nekonaru/getta/releases/download/${tag}/${path.basename(assetPath)}`;

const platforms = {
	'windows-x86_64': {
		signature: sigOf(windowsSig),
		url: urlFor(windowsSig.replace(/\.sig$/, ''))
	},
	'linux-x86_64': {
		signature: sigOf(linuxSig),
		url: urlFor(linuxSig.replace(/\.sig$/, ''))
	}
};

for (const sig of macSigs) {
	const arch = sig.includes('aarch64') ? 'darwin-aarch64' : 'darwin-x86_64';
	platforms[arch] = {
		signature: sigOf(sig),
		url: urlFor(sig.replace(/\.sig$/, ''))
	};
}

const latest = {
	version,
	notes: 'See the full release notes on GitHub Releases.',
	pub_date: new Date().toISOString(),
	platforms
};

fs.writeFileSync('latest.json', JSON.stringify(latest, null, '\t'));
console.log(JSON.stringify(latest.platforms, null, '\t'));
