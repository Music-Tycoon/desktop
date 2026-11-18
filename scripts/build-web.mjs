// Compila la app web (../musicopoly-app) en ./web-dist para meterla dentro de la app de escritorio.
// No toca musicopoly-app/dist (esa es la que publica el servidor).
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const app = fileURLToPath(new URL('../../musicopoly-app/', import.meta.url));
const out = fileURLToPath(new URL('../web-dist/', import.meta.url));
const r = spawnSync('npx', ['vite', 'build', '--outDir', out, '--emptyOutDir'], { cwd: app, stdio: 'inherit', env: process.env });
process.exit(r.status ?? 1);
