import { useMemo } from 'react'

export default function MarkdownView({ html }: { html: string }) {
  const __html = useMemo(() => {
    if (!html) return ''
    // Basic post-processing
    return html
  }, [html])

  if (!html) return null

  return (
    <div className="markdown-body" dangerouslySetInnerHTML={{ __html: html }} />
  )
}
