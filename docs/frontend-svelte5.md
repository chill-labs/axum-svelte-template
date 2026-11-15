# Frontend Architecture (Svelte 5 + SvelteKit + Tailwind v4)

## 1. SvelteKit `adapter-static` & Vite Proxy

`svelte-app/vite.config.ts` compiles the frontend as a static CSR SPA (`build/` with `200.html` fallback and `.br`/`.gz` precompression):
- **Development (`npm run dev` on `:5173`)**: Vite proxies `/api`, `/auth/google/callback`, `/logo`, `/guides`, `/js`, `/robots.txt`, and `/sitemap.xml` to Axum on `http://127.0.0.1:3000` (configurable via `APP__SERVER__PORT`).
- **Production (`crates/server`)**: Axum serves `SVELTE_SITE_ROOT=svelte-app/build`, `PUBLIC_ASSETS_ROOT=public`, and injects `window.__APP_BOOTSTRAP__` into `<!--APP_BOOTSTRAP_SLOT-->` of `app.html`.

---

## 2. Svelte 5 Runes State Management (`svelte-app/src/lib/state/`)

- Use Svelte 5 Runes (`$state`, `$derived`, `$effect`, `$props`) in `.svelte` components and `.svelte.ts` store classes.
- `theme.svelte.ts` (`uiPrefs`) reads initial locale (`vi` / `en`) and theme (`light` / `dark`) from `window.__APP_BOOTSTRAP__` and `document.documentElement`, updating cookies (`lang`, `theme`) cleanly without SSR/hydration flash.
- `toast.svelte.ts` manages transient toast notifications translated through `uiPrefs.err(code)`.

---

## 3. Shared Soft UI Styling (`style/input.css`)

- `style/input.css` defines Tailwind CSS v4 `@theme` design tokens (`--color-primary*`, `--color-base-main`, `--color-base-sub`, `--color-base-content*`, `--color-bg-main`, `--color-text-main`, gradients, shadows), `.dark` overrides, `.glass-card`, `.soft-card`, utility animations, and mobile `<input>` auto-zoom prevention.
- `sharedStylesPlugin()` in `svelte-app/vite.config.ts` injects `style/input.css` into `svelte-app/src/app.css` at `/* __SHARED_STYLE_TOKENS__ */` on every dev/build pass while keeping `style/input.css` valid standalone.

---

## 4. Bilingual i18n & Contextual Page Guides

- **i18n (`svelte-app/src/lib/i18n/{types.ts,en.ts,vi.ts,index.ts}`)**: Strictly typed dictionaries covering navigation, forms, workspace RBAC labels, and all `ErrorCode` translations.
- **Page Guides (`public/js/page-guide.js` + `public/guides/meta.json`)**: Zero-polling History API listener rendering a collapsible bottom-right Help button on matching routes (and listening to `[data-open-page-guide]`) that opens `/guides/*.html` inside a native `<dialog closedby="any">` modal. Zero `/pkg` paths are used.
