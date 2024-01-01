import tailwind from '@astrojs/tailwind';
import { defineConfig } from 'astro/config';
import react from '@astrojs/react';
import relativeLinks from 'astro-relative-links';

export default defineConfig({
	integrations: [tailwind(), react(), relativeLinks()],
});
