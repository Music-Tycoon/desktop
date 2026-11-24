// Compila la app web (../musicopoly-app) en ./web-dist para meterla dentro de la app de escritorio.
// No toca musicopoly-app/dist (esa es la que publica el servidor).
// Llama a vite por su bin con node (multiplataforma: npx necesita shell en Windows).
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';

const app = fileURLToPath(new URL('../../musicopoly-app/', import.meta.url));
const out = fileURLToPath(new URL('../web-dist/', import.meta.url));
const vitePkg = createRequire(new URL('../../musicopoly-app/package.json', import.meta.url)).resolve('vite/package.json');
const vite = fileURLToPath(new URL('./bin/vite.js', `file://${vitePkg}`));
const r = spawnSync(process.execPath, [vite, 'build', '--outDir', out, '--emptyOutDir'], { cwd: app, stdio: 'inherit', env: process.env });
process.exit(r.status ?? 1);
