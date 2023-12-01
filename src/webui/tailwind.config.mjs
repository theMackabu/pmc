/** @type {import('tailwindcss').Config} */
export default {
	content: ['./src/**/*.{astro,html,js,jsx,md,mdx,svelte,ts,tsx,vue}'],
	theme: {
		extend: {
			fontFamily: {
				sans: ['Inter var', ...require('tailwindcss/defaultTheme').fontFamily.sans],
			},
		},
	},
	plugins: [require('@tailwindcss/forms')],
};
