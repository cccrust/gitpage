import { useEffect, useRef } from 'react'

export default function MarkdownView({ html }: { html: string }) {
  const ref = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!html || !ref.current) return
    const el = ref.current

    const tryKaTeX = () => {
      const katex = (window as any).katex
      if (!katex) { setTimeout(tryKaTeX, 200); return }
      el.querySelectorAll<HTMLElement>('.math-inline').forEach(el => {
        const raw = (el.textContent || '').replace(/^\\\(|\\\)$/g, '')
        try { katex.render(raw, el, { displayMode: false, throwOnError: false }) } catch {}
      })
      el.querySelectorAll<HTMLElement>('.math-display').forEach(el => {
        const raw = (el.textContent || '').replace(/^\\\[|\\\]$/g, '')
        try { katex.render(raw, el, { displayMode: true, throwOnError: false }) } catch {}
      })
    }

    const tryMermaid = () => {
      const mermaid = (window as any).mermaid
      if (!mermaid) { setTimeout(tryMermaid, 200); return }
      try { mermaid.initialize({ startOnLoad: false }) } catch {}
      el.querySelectorAll<HTMLElement>('pre > code.language-mermaid').forEach(el => {
        const pre = el.closest('pre')
        if (!pre) return
        pre.className = 'mermaid'
        pre.innerHTML = el.textContent || ''
      })
      const mermaidEls = el.querySelectorAll<HTMLElement>('pre.mermaid')
      if (mermaidEls.length > 0) {
        mermaid.run({ nodes: [...mermaidEls] }).catch(() => {})
      }
    }

    tryKaTeX()
    tryMermaid()
  }, [html])

  if (!html) return null

  return (
    <div ref={ref} className="markdown-body" dangerouslySetInnerHTML={{ __html: html }} />
  )
}