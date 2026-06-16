import { useEffect, useState } from 'react'
import { useParams, Link } from 'react-router-dom'
import { getRepo, listCommits, type Repo, type CommitInfo } from '../api'
import Spinner from '../components/Spinner'

export default function CommitsPage() {
  const { id, branch } = useParams<{ id: string; branch: string }>()
  const [repo, setRepo] = useState<Repo | null>(null)
  const [commits, setCommits] = useState<CommitInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [err, setErr] = useState('')

  useEffect(() => {
    if (!id) return
    const numId = parseInt(id)
    if (isNaN(numId)) { setErr('ID 無效'); setLoading(false); return }
    const br = branch || 'main'

    setLoading(true)
    getRepo(numId)
      .then(async r => {
        setRepo(r.repo)
        const uname = r.username
        const commitsRes = await listCommits(uname, r.repo.name, br)
        setCommits(commitsRes.commits)
      })
      .catch(e => setErr(e.message))
      .finally(() => setLoading(false))
  }, [id, branch])

  if (loading) return <Spinner />
  if (err) return <div className="error-box">{err}</div>
  if (!repo) return <div className="error-box">找不到</div>

  return (
    <div className="repo-page">
      <div className="repo-header">
        <div className="breadcrumb">
          <Link to="/">~</Link> / <Link to={`/repo/${repo.id}`}>{repo.name}</Link> / commits
        </div>
      </div>

      <div className="commit-list">
        {commits.length === 0 ? (
          <div className="empty-state"><p>No commits</p></div>
        ) : (
          commits.map(c => (
            <div key={c.sha} className="commit-entry">
              <div className="sha">{c.sha}</div>
              <div className="msg">{c.message.split('\n')[0]}</div>
              <div className="meta">
                <span>{c.author}</span>
                <span>{c.time}</span>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  )
}
