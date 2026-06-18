import { useState } from 'react'
import { useParams, useNavigate, Link } from 'react-router-dom'
import { createIssue, getRepo } from '../api'

export default function IssueNew() {
  const { id } = useParams<{ id: string }>()
  const nav = useNavigate()
  const [title, setTitle] = useState('')
  const [body, setBody] = useState('')
  const [submitting, setSubmitting] = useState(false)
  const [err, setErr] = useState('')

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!title.trim() || !id) return
    setSubmitting(true)
    setErr('')
    try {
      const numId = parseInt(id)
      const repoData = await getRepo(numId)
      const res = await createIssue(numId, title.trim(), body || undefined)
      nav(`/repo/${repoData.repo.id}/issues/${res.issue.issue.number}`)
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : '建立失敗')
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <div className="new-repo-page">
      <div className="breadcrumb">
        <Link to={`/repo/${id}`}>倉庫</Link> / <Link to={`/repo/${id}/issues`}>Issues</Link> / <strong>New Issue</strong>
      </div>
      <h2>New Issue</h2>
      {err && <div className="error-box">{err}</div>}
      <form onSubmit={handleSubmit}>
        <label>Title</label>
        <input type="text" value={title} onChange={e => setTitle(e.target.value)} placeholder="Issue title" required />
        <label>Description</label>
        <textarea value={body} onChange={e => setBody(e.target.value)} placeholder="Optional description" rows={6} />
        <button type="submit" className="btn" disabled={submitting || !title.trim()} style={{ marginTop: 12 }}>
          {submitting ? 'Submitting...' : 'Create Issue'}
        </button>
      </form>
    </div>
  )
}
