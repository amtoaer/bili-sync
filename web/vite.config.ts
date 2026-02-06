import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],
	server: {
		proxy: {
			'/api/ws': {
				target: 'ws://localhost:12345',
				ws: true,
				rewriteWsOrigin: true
			},
			'/api': 'http://localhost:12345'
		},
		host: true
	}
});
