import fs from 'node:fs';
import path from 'node:path';
import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig, loadEnv, type Plugin } from 'vite';

/**
 * Injects shared Soft UI `@theme` tokens, `.dark` overrides, and utility classes
 * from `../style/input.css` into `src/app.css` without scanning Rust crates.
 */
function sharedStylesPlugin(): Plugin {
  const sharedCssPath = path.resolve(import.meta.dirname, '../style/input.css');
  const marker = '/* __SHARED_STYLE_TOKENS__ */';

  return {
    name: 'shared-styles-injector',
    enforce: 'pre',
    transform(code, id) {
      const cleanId = id.split('?', 1)[0];
      if (!cleanId.endsWith('/src/app.css') || !code.includes(marker)) {
        return null;
      }
      if (!fs.existsSync(sharedCssPath)) {
        return { code: code.replace(marker, ''), map: null };
      }
      this.addWatchFile(sharedCssPath);
      const sharedCss = fs
        .readFileSync(sharedCssPath, 'utf8')
        .replace(/^@import\s+"tailwindcss";\s*$/gm, '')
        .replace(/^@source\s+"[^"]+";\s*$/gm, '');

      return {
        code: code.replace(marker, sharedCss),
        map: null
      };
    }
  };
}

const proxiedPrefixes = [
  '/api',
  '/auth/google/callback',
  '/logo',
  '/guides',
  '/js',
  '/robots.txt',
  '/sitemap.xml'
];

export default defineConfig(({ mode }) => {
  const rootEnv = loadEnv(mode, path.resolve(import.meta.dirname, '..'), '');
  const localEnv = loadEnv(mode, '.', '');
  const env = { ...rootEnv, ...localEnv };
  const backendPort = env.APP__SERVER__PORT || env.APP_PORT || '3000';
  const backendOrigin = `http://127.0.0.1:${backendPort}`;

  return {
    plugins: [
      sharedStylesPlugin(),
      tailwindcss(),
      sveltekit({
        preprocess: vitePreprocess(),
        adapter: adapter({
          pages: 'build',
          assets: 'build',
          fallback: '200.html',
          precompress: true,
          strict: false
        })
      })
    ],
    server: {
      port: 5173,
      fs: {
        allow: ['..']
      },
      proxy: Object.fromEntries(
        proxiedPrefixes.map((prefix) => [
          prefix,
          {
            target: backendOrigin,
            changeOrigin: false,
            ws: false
          }
        ])
      )
    }
  };
});
