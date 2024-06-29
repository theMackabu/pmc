import react from '@astrojs/react';
import relativeLinks from './links';
import tailwind from '@astrojs/tailwind';
import { defineConfig } from 'astro/config';

export default defineConfig({
	build: { format: 'file', assets: 'assets' },
	integrations: [tailwind(), react(), relativeLinks()]
});
