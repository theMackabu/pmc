import tailwind from '@astrojs/tailwind';
import { defineConfig } from 'astro/config';
import react from '@astrojs/react';
import relativeLinks from './links';

export default defineConfig({
	build: { format: 'file', assets: 'assets' },
	integrations: [tailwind(), react(), relativeLinks()],
});
