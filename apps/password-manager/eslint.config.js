import { fileURLToPath } from 'node:url';
import { createSvelteConfig } from '../../eslint.shared-config.js';
import svelteConfig from './svelte.config.js';

const gitignorePath = fileURLToPath(new URL('./.gitignore', import.meta.url));
const tsconfigRootDir = fileURLToPath(new URL('.', import.meta.url));

export default createSvelteConfig({ gitignorePath, tsconfigRootDir, svelteConfig });
