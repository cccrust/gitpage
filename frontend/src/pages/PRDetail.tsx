import { useEffect, useState } from 'react'
import { useParams, Link } from 'react-router-dom'
import { getPR, listComments, addComment, mergePR, getPRDiff, updatePR, isLoggedIn } from '../api'
import type { PullRequestWithAuthor, IssueComment, DiffEntry } from '../api'
import Spinner from '../components/Spinner'

export default function PRDetail() {
  const { id, prNumber } = useParams<{ id: string; prNumber: string }>()
  const [data, setData] = useState<PullRequestWithAuthor | null>(null)
  const [comments, setComments] = useState<IssueComment[]>([])
  const [diff, setDiff] = useState<DiffEntry[]>([])
  const [loading, setLoading] = useState(true)
  const [err, setErr] = useState('')
  const [commentBody, setCommentBody] = useState('')
  const [submitting, setSubmitting] = useState(false)
  const [merging, setMerging] = useState(false)
  const [showDiff, setShowDiff] = useState(false)

  const load = () => {
    if (!id || !prNumber) return
    const numId = parseInt(id)
    const numPr = parseInt(prNumber)
    if (isNaN(numId) || isNaN(numPr)) { setErr('ID 無效'); setLoading(false); return }
    setLoading(true)
    Promise.all([
      getPR(numId, numPr),
      listComments(numId, numPr),
      getPRDiff(numId, numPr),
    ])
      .then(([prRes, commentRes, diffRes]) => {
        setData(prRes.pr)
        setComments(commentRes.comments)
        setDiff(diffRes.diff)
      })
      .catch(e => setErr(e.message))
      .finally(() => setLoading(false))
  }

  useEffect(() => { load() }, [id, prNumber])

  const handleMerge = async () => {
    if (!id || !prNumber || !data || data.pr.state !== 'open') return
    if (!confirm('確定合併此 Pull Request？')) return
    setMerging(true)
    try {
      await mergePR(parseInt(id), parseInt(prNumber))
      load()
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : '合併失敗')
    } finally {
      setMerging(false)
    }
  }

  const handleClose = async () => {
    if (!id || !prNumber || !data || data.pr.state !== 'open') return
    try {
      await updatePR(parseInt(id), parseInt(prNumber), { state: 'closed' })
      load()
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : '關閉失敗')
    }
  }

  const handleAddComment = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!commentBody.trim() || !id || !prNumber) return
    setSubmitting(true)
    try {
      await addComment(parseInt(id), parseInt(prNumber), commentBody.trim())
      setCommentBody('')
      const res = await listComments(parseInt(id), parseInt(prNumber))
      setComments(res.comments)
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : '留言失敗')
    } finally {
      setSubmitting(false)
    }
  }

  if (loading) return <Spinner />
  if (err) return <div className="error-box">{err}</div>
  if (!data) return <div className="error-box">PR 不存在</div>

  const pr = data.pr
  const isOpen = pr.state === 'open'
  const isMerged = pr.state === 'merged'

  return (
    <div className="page" style={{ padding: '16px 0 80px' }}>
      <div className="breadcrumb">
        <Link to={`/repo/${id}`}>倉庫</Link> / <Link to={`/repo/${id}/pulls`}>PRs</Link> / <strong>#{pr.number}</strong>
      </div>

      <div style={{ marginTop: 16 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
          <span className={`state-badge ${pr.state}`}>{pr.state}</span>
          <h2 style={{ fontSize: 20, fontWeight: 600 }}>{pr.title}</h2>
        </div>
        <div style={{ fontSize: 13, color: '#7c7c7c', marginBottom: 12 }}>
          {data.author_username} wants to merge {data.head_repo_owner}/{data.head_repo_name}:{pr.head_ref} into {pr.base_ref}
        </div>
        <div style={{ fontSize: 13, color: '#7c7c7c', marginBottom: 16 }}>
          opened {new Date(pr.created_at).toLocaleDateString()}
          {pr.closed_at && ` · closed ${new Date(pr.closed_at).toLocaleDateString()}`}
          {pr.merged_at && ` · merged ${new Date(pr.merged_at).toLocaleDateString()}`}
          {pr.merge_commit_sha && ` · merge commit: ${pr.merge_commit_sha}`}
        </div>

        {pr.body && (
          <div style={{ padding: '12px 16px', border: '1px solid #e5e5e5', borderRadius: 6, marginBottom: 16, whiteSpace: 'pre-wrap', fontSize: 14, lineHeight: 1.7 }}>
            {pr.body}
          </div>
        )}

        {isLoggedIn() && isOpen && (
          <div style={{ display: 'flex', gap: 8, marginBottom: 16 }}>
            <button onClick={handleMerge} className="btn-sm" disabled={merging} style={{ background: '#2da44e', color: '#fff', borderColor: '#2da44e' }}>
              {merging ? 'Merging...' : 'Merge Pull Request'}
            </button>
            <button onClick={handleClose} className="btn-sm">Close</button>
          </div>
        )}

        {isMerged && (
          <div style={{ padding: '10px 14px', background: '#f0fff4', border: '1px solid #2da44e', borderRadius: 6, marginBottom: 16, fontSize: 14, color: '#166534' }}>
            Pull request successfully merged and closed.
          </div>
        )}
      </div>

      <div style={{ marginTop: 16 }}>
        <button onClick={() => setShowDiff(!showDiff)} className="btn-sm" style={{ marginBottom: 12 }}>
          {showDiff ? 'Hide' : 'Show'} Diff ({diff.length} files)
        </button>

        {showDiff && diff.length > 0 && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4, marginBottom: 16 }}>
            {diff.map((d, i) => (
              <div key={i} style={{
                display: 'flex', alignItems: 'center', gap: 8,
                padding: '8px 12px', border: '1px solid #e5e5e5', borderRadius: 4, fontSize: 13
              }}>
                <span style={{
                  fontWeight: 600, fontSize: 11, padding: '1px 6px', borderRadius: 3,
                  background: d.status === 'added' ? '#dff0d8' : d.status === 'deleted' ? '#f2dede' : '#f5f5f5',
                  color: d.status === 'added' ? '#3c763d' : d.status === 'deleted' ? '#a94442' : '#666'
                }}>
                  {d.status}
                </span>
                <span style={{ fontFamily: 'monospace' }}>{d.new_path || d.old_path}</span>
              </div>
            ))}
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
