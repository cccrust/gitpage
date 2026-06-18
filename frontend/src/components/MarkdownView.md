# MarkdownView — Client-Side HTML Rendering Pipeline

## Overview

`MarkdownView` is a React component that takes a pre-rendered HTML string (produced by the Rust backend from Markdown source) and renders it safely into the DOM. It then applies two client-side post-processing steps: KaTeX for mathematical notation and Mermaid for diagrams.

The component is used primarily by `RepoPage` (for rendering README files) and `FileViewPage` (for rendering Markdown files viewed inside a repository).

## Backend Rendering vs Frontend Rendering

This project takes a **server-rendered Markdown** approach, unlike many Git platforms that render Markdown on the client side.

### The Backend Side

When the API endpoint `/api/:username/:repo/readme` or `/api/:username/:repo/blob` is called with a Markdown file:

1. The backend (Rust, using `pulldown_cmark`) parses the raw Markdown into an HTML string.
2. The backend wraps math expressions with special HTML classes:
   - Inline math `$...$` becomes `<span class="math-inline">\(...\)</span>`
   - Display math `$$...$$` becomes `<div class="math-display">\[...\]</div>`
3. Mermaid code blocks (```` ```mermaid ````) are rendered as `<pre><code class="language-mermaid">...</code></pre>`.
4. The rendered HTML is returned in the `rendered` field of the response JSON.

### The Backend Renders Markdown, Not the Frontend

This is an important architectural decision. The alternative would be to ship a Markdown parser like `marked` or `remark` to the client, parse raw `.md` content in the browser, and generate HTML on the client side. The server-rendered approach:

- **Reduces bundle size** — no need to ship a Markdown parser to the client.
- **Ensures consistency** — the same `pulldown_cmark` parser handles all Markdown, avoiding differences between client and server renderers.
- **Simplifies the frontend** — `MarkdownView` only needs `dangerouslySetInnerHTML` and the KaTeX/Mermaid post-processors.
- **Moves rendering cost to the server** — each Markdown view triggers an API call, but the server-side rendering is fast (pure Rust) and the result can be cached.

## The `dangerouslySetInnerHTML` Pattern

The core of the component is:

```tsx
<div ref={ref} className="markdown-body" dangerouslySetInnerHTML={{ __html: html }} />
```

`dangerouslySetInnerHTML` is React's escape hatch for setting raw HTML. By default, React escapes all content to prevent XSS attacks. This prop bypasses that protection.

### Why This Is Necessary

Markdown rendering produces HTML elements (`<h1>`, `<p>`, `<a>`, `<code>`, `<pre>`, `<img>`, etc.) that must be injected into the DOM as real DOM nodes. React's JSX cannot express arbitrary HTML strings as virtual DOM elements — the only way to insert raw HTML is `dangerouslySetInnerHTML` or `ref.current.innerHTML`.

### XSS Prevention

There are two layers of defense:

1. **Backend sanitization** — the Rust `pulldown_cmark` parser produces safe HTML. It does not allow raw `<script>` tags, `onclick` attributes, or `javascript:` URLs unless explicitly configured. The backend can optionally run the output through an HTML sanitizer (like `ammonia`) to strip dangerous elements.
2. **No client-side sanitization** — the frontend does not run a sanitizer (like DOMPurify) on the received HTML. It trusts that the backend has already sanitized the output. If the backend were compromised or misconfigured, XSS would be possible.

This is a **trust-the-backend** model. The frontend assumes the API response is safe because it came from its own backend, not from user input directly.

## KaTeX Mathematical Expression Rendering

After the HTML is in the DOM, the component searches for elements with the classes `math-inline` and `math-display` and renders them with KaTeX.

### The Polling Pattern

KaTeX is loaded as a global script (via a `<script>` tag in `index.html` or loaded dynamically). The component cannot know when the KaTeX library has finished loading. To handle this:

```tsx
const tryKaTeX = () => {
  const katex = (window as any).katex
  if (!katex) { setTimeout(tryKaTeX, 200); return }
  // ... render math
}
```

This **polling loop** retries every 200ms until `window.katex` is defined. The same pattern is used for Mermaid. This is a pragmatic solution for a component that does not control script loading. A more robust alternative would be using a module import (`import katex from 'katex'`) and letting the bundler handle the dependency, but that requires the KaTeX npm package as a dependency.

### Rendering Logic

- For inline math: the text content of each `.math-inline` element has the `\(` and `\)` delimiters stripped, then is passed to `katex.render()` with `displayMode: false`.
- For display math: the `\[` and `\]` delimiters are stripped from `.math-display` elements, and `katex.render()` is called with `displayMode: true`.

Both calls use `throwOnError: false` so that invalid LaTeX renders as raw text rather than breaking the page.

## Mermaid Diagram Rendering

Mermaid diagrams are handled similarly but with an extra transformation step:

1. The component finds all `<pre><code class="language-mermaid">` elements.
2. It transforms the `<pre>` element: sets `className = 'mermaid'`, replaces the inner HTML with just the text content (the diagram definition).
3. After all elements are transformed, it calls `mermaid.run({ nodes: [...] })` to render them all at once.

This two-step approach is necessary because Mermaid expects its diagrams to be in elements with class `mermaid`, not inside `<pre><code>` blocks. The transformation converts the standard Markdown fenced code block output into the format Mermaid's runtime expects.

The `mermaid.initialize({ startOnLoad: false })` call prevents Mermaid from auto-running on page load, since the component controls when rendering happens.

## The `useEffect` and Cleanup

The entire post-processing pipeline runs inside a `useEffect` that depends on the `html` prop. When the prop changes (e.g., navigating from one README to another):

1. React re-renders the `<div>` with the new HTML.
2. The effect runs and re-applies KaTeX and Mermaid transformations.
3. Previous transformations are lost because the innerHTML is fully replaced — there is no cleanup function needed.

The component does not cache rendered state. Each change to the `html` prop triggers a full re-render and re-transformation.

## Summary of the Pipeline

```
Backend (.md file)
  → pulldown_cmark renders Markdown → HTML
  → Backend wraps math in .math-inline / .math-display
  → Backend wraps mermaid in pre > code.language-mermaid
  → JSON response with rendered HTML string
    → Frontend MarkdownView receives html prop
      → dangerouslySetInnerHTML inserts HTML into DOM
        → KaTeX polling finds math elements and renders equations
        → Mermaid polling finds mermaid blocks and renders diagrams
```

## Reference: Wiki

See `_wiki/pulldown-cmark.md` for the backend Markdown-to-HTML rendering pipeline. See `_wiki/markdown.md` for the overall Markdown feature design.
