import { useEffect, useState } from 'react'
import { useParams, Link, useNavigate } from 'react-router-dom'
import { getIssue, listComments, addComment, updateIssue, deleteIssue, isLoggedIn } from '../api'
import type { IssueWithAuthor, IssueComment } from '../api'
import Spinner from '../components/Spinner'

export default function IssueDetail() {
  const { id, issueNumber } = useParams<{ id: string; issueNumber: string }>()
  const nav = useNavigate()
  const [data, setData] = useState<IssueWithAuthor | null>(null)
  const [comments, setComments] = useState<IssueComment[]>([])
  const [loading, setLoading] = useState(true)
  const [err, setErr] = useState('')
  const [commentBody, setCommentBody] = useState('')
  const [submitting, setSubmitting] = useState(false)

  const load = () => {
    if (!id || !issueNumber) return
    const numId = parseInt(id)
    const numIssue = parseInt(issueNumber)
    if (isNaN(numId) || isNaN(numIssue)) { setErr('ID 無效'); setLoading(false); return }
    setLoading(true)
    Promise.all([
      getIssue(numId, numIssue),
      listComments(numId, numIssue),
    ])
      .then(([issueRes, commentRes]) => {
        setData(issueRes.issue)
        setComments(commentRes.comments)
      })
      .catch(e => setErr(e.message))
      .finally(() => setLoading(false))
  }

  useEffect(() => { load() }, [id, issueNumber])

  const handleToggleState = async () => {
    if (!data || !id || !issueNumber) return
    const newState = data.issue.state === 'open' ? 'closed' : 'open'
    try {
      await updateIssue(parseInt(id), parseInt(issueNumber), { state: newState })
      load()
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : '操作失敗')
    }
  }

  const handleDelete = async () => {
    if (!id || !issueNumber || !confirm('確定刪除此 Issue？')) return
    try {
      await deleteIssue(parseInt(id), parseInt(issueNumber))
      nav(`/repo/${id}/issues`)
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : '刪除失敗')
    }
  }

  const handleAddComment = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!commentBody.trim() || !id || !issueNumber) return
    setSubmitting(true)
    try {
      await addComment(parseInt(id), parseInt(issueNumber), commentBody.trim())
      setCommentBody('')
      const res = await listComments(parseInt(id), parseInt(issueNumber))
      setComments(res.comments)
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : '留言失敗')
    } finally {
      setSubmitting(false)
    }
  }

  if (loading) return <Spinner />
  if (err) return <div className="error-box">{err}</div>
  if (!data) return <div className="error-box">Issue 不存在</div>

  const issue = data.issue
  const isOpen = issue.state === 'open'

  return (
    <div className="page" style={{ padding: '16px 0 80px' }}>
      <div className="breadcrumb">
        <Link to={`/repo/${id}`}>倉庫</Link> / <Link to={`/repo/${id}/issues`}>Issues</Link> / <strong>#{issue.number}</strong>
      </div>

      <div style={{ marginTop: 16 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
          <span className={`state-badge ${issue.state}`}>{issue.state}</span>
          <h2 style={{ fontSize: 20, fontWeight: 600 }}>{issue.title}</h2>
        </div>
        <div style={{ fontSize: 13, color: '#7c7c7c', marginBottom: 16 }}>
          {data.author_username} opened · {new Date(issue.created_at).toLocaleDateString()}
          {issue.closed_at && ` · closed ${new Date(issue.closed_at).toLocaleDateString()}`}
        </div>

        {data.labels.length > 0 && (
          <div style={{ display: 'flex', gap: 4, marginBottom: 12 }}>
            {data.labels.map(l => (
              <span key={l.id} style={{
                display: 'inline-block', padding: '2px 8px', borderRadius: 3,
                fontSize: 12, fontWeight: 600, background: `#${l.color}22`, color: `#${l.color}`
              }}>{l.name}</span>
            ))}
          </div>
        )}

        {issue.body && (
          <div style={{ padding: '12px 16px', border: '1px solid #e5e5e5', borderRadius: 6, marginBottom: 16, whiteSpace: 'pre-wrap', fontSize: 14, lineHeight: 1.7 }}>
            {issue.body}
          </div>
        )}

        {isLoggedIn() && (
          <div style={{ display: 'flex', gap: 8, marginBottom: 16 }}>
            <button onClick={handleToggleState} className="btn-sm">
              {isOpen ? 'Close Issue' : 'Reopen Issue'}
            </button>
            <button onClick={handleDelete} className="btn-sm danger">Delete</button>
          </div>
        )}
      </div>

      <div style={{ marginTop: 24 }}>
        <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 12, color: '#7c7c7c' }}>
          Comments ({comments.length})
        </h3>

        {comments.length === 0 ? (
          <div className="empty-state"><p>No comments yet</p></div>
        ) : (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            {comments.map(c => (
              <div key={c.id} style={{ padding: '10px 14px', border: '1px solid #e5e5e5', borderRadius: 6 }}>
                <div style={{ fontSize: 12, color: '#7c7c7c', marginBottom: 4 }}>
                  <strong>{c.author_username}</strong> · {new Date(c.created_at).toLocaleString()}
                </div>
                <div style={{ fontSize: 14, whiteSpace: 'pre-wrap' }}>{c.body}</div>
              </div>
            ))}
          </div>
        )}

        {isLoggedIn() && (
          <form onSubmit={handleAddComment} style={{ marginTop: 12 }}>
            <textarea
              value={commentBody}
              onChange={e => setCommentBody(e.target.value)}
              placeholder="Leave a comment"
              rows={3}
              required
            />
            <button type="submit" className="btn-sm" disabled={submitting || !commentBody.trim()} style={{ marginTop: 8 }}>
              {submitting ? 'Submitting...' : 'Comment'}
            </button>
          </form>
        )}
      </div>
    </div>
  )
}
