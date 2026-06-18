import { useState, useEffect } from 'react'
import { useParams, useNavigate, Link } from 'react-router-dom'
import { createPR, getRepo } from '../api'

export default function PRNew() {
  const { id } = useParams<{ id: string }>()
  const nav = useNavigate()
  const [title, setTitle] = useState('')
  const [body, setBody] = useState('')
  const [baseRef, setBaseRef] = useState('main')
  const [headRef, setHeadRef] = useState('')
  const [submitting, setSubmitting] = useState(false)
  const [err, setErr] = useState('')

  useEffect(() => {
    if (!id) return
    const numId = parseInt(id)
    if (isNaN(numId)) return
    getRepo(numId).then(r => {
      setBaseRef(r.repo.default_branch)
    }).catch(e => setErr(e.message))
  }, [id])

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!title.trim() || !headRef.trim() || !id) return
    setSubmitting(true)
    setErr('')
    try {
      const numId = parseInt(id)
      const res = await createPR(numId, title.trim(), numId, headRef.trim(), baseRef.trim(), body || undefined)
      nav(`/repo/${numId}/pulls/${res.pr.pr.number}`)
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : '建立失敗')
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <div className="new-repo-page">
      <div className="breadcrumb">
        <Link to={`/repo/${id}`}>倉庫</Link> / <Link to={`/repo/${id}/pulls`}>PRs</Link> / <strong>New Pull Request</strong>
      </div>
      <h2>New Pull Request</h2>
      {err && <div className="error-box">{err}</div>}
      <form onSubmit={handleSubmit}>
        <label>Base branch</label>
        <input type="text" value={baseRef} onChange={e => setBaseRef(e.target.value)} placeholder="main" />
        <label>Head branch</label>
        <input type="text" value={headRef} onChange={e => setHeadRef(e.target.value)} placeholder="feature-branch" required />
        <label>Title</label>
        <input type="text" value={title} onChange={e => setTitle(e.target.value)} placeholder="PR title" required />
        <label>Description</label>
        <textarea value={body} onChange={e => setBody(e.target.value)} placeholder="Optional description" rows={6} />
        <button type="submit" className="btn" disabled={submitting || !title.trim() || !headRef.trim()} style={{ marginTop: 12 }}>
          {submitting ? 'Creating...' : 'Create Pull Request'}
        </button>
      </form>
    </div>
  )
}
