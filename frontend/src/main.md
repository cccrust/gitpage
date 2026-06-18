# main.tsx — Vite Entry Point

## Overview

`main.tsx` is the application's entry point as configured in `vite.config.ts` and `index.html`. It is responsible for mounting the React component tree into the DOM and importing the global stylesheet. This is the only file that interacts directly with the DOM API.

## Vite Entry Point Convention

Vite expects an explicit JavaScript/TypeScript entry point referenced from `index.html` via a `<script type="module">` tag:

```html
<!-- frontend/index.html -->
<div id="root"></div>
<script type="module" src="/src/main.tsx"></script>
```

When Vite builds the production bundle, it starts from `main.tsx`, traverses the import graph (App → Layout, pages, components → api.ts), and tree-shakes unused exports. The output is a single JS bundle (plus chunked dependencies) written to `frontend/dist/`.

The `createRoot` API is the React 18+ replacement for `ReactDOM.render`:

```
createRoot(document.getElementById('root')!).render(<App />)
```

The non-null assertion (`!`) assumes the `#root` div exists in `index.html`. If it is missing, the application crashes at startup — there is no defensive fallback.

## React.StrictMode Wrapping

The `<App />` component is wrapped in `<StrictMode>`:

```
<StrictMode>
  <App />
</StrictMode>
```

`StrictMode` is a development-only wrapper that:

- **Double-renders** every component — in development, effects run twice to detect impure render logic and missing cleanup functions.
- **Detects unsafe lifecycle methods** — warns about legacy class component APIs.
- **Warns about legacy string refs** and `findDOMNode` usage.
- **Does nothing in production** — the `StrictMode` wrapper is stripped from the production build, so there is zero runtime cost in the deployed application.

The double-render behavior in development is a frequent source of confusion. `useEffect` callbacks that make API calls will fire twice, triggering two network requests. This is intentional — it exposes components that rely on side-effect timing rather than declarative state. All pages in this project handle this correctly because the `useEffect` cleanup is either absent (no-op on re-fire) or properly implemented.

## CSS Import at Root Level

The line:

```
import './index.css'
```

imports the global stylesheet as a side-effect import. Vite's CSS handling:

- **In development** — CSS is injected via JavaScript as a `<style>` tag for hot module replacement (HMR). Editing `index.css` instantly updates the browser without a full page reload.
- **In production** — CSS is extracted into a separate `.css` file (code-split by entry point) and loaded via a `<link>` tag in the built `index.html`.

There are no CSS modules, no CSS-in-JS libraries, and no preprocessor (Sass/Less/PostCSS) configured. The single `index.css` file contains all styles using CSS custom properties for the dark theme. Components do not import their own stylesheets — all styling is global and class-name-based.

## Design Rationale

The minimal `main.tsx` reflects the project's overall philosophy: keep the frontend simple. There is no:

- Service worker registration (no PWA support)
- Polyfill loading (target modern browsers)
- Analytics initialization
- Error boundary wrapping (errors bubble up to the browser console)
- Provider component mounting (no Redux `<Provider>`, no React Query `<QueryClientProvider>`, no `<AuthProvider>`)

Everything that could be considered "app initialization" is handled by individual page components via `useEffect` on mount. The entry point's sole job is to render the root component and import the stylesheet.
