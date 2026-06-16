import { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import { listRepos, type Repo, isLoggedIn } from '../api'

export default function Dashboard() {
  const [repos, setRepos] = useState<Repo[]>([])
  const [loading, setLoading] = useState(true)
  const [err, setErr] = useState('')
  const loggedIn = isLoggedIn()

  useEffect(() => {
    if (!loggedIn) {
      setLoading(false)
      return
    }
    listRepos()
      .then(r => setRepos(r.repos))
      .catch(e => setErr(e.message))
      .finally(() => setLoading(false))
  }, [loggedIn])

  if (!loggedIn) {
    return (
      <div className="dashboard">
        <div className="empty-state">
          <h1 style={{ fontSize: 24, marginBottom: 8 }}>gitpage</h1>
          <p>Self-hosted Git platform</p>
          <p style={{ marginTop: 16 }}>
            <Link to="/login" style={{ textDecoration: 'underline' }}>Login</Link>
            {' or '}
            <Link to="/register" style={{ textDecoration: 'underline' }}>Register</Link>
          </p>
        </div>
      </div>
    )
  }

  if (loading) return <div className="loading">Loading...</div>
  if (err) return <div className="error-box">{err}</div>

  return (
    <div className="dashboard">
      <div className="head">
        <h1>Repositories</h1>
        <Link to="/new" className="btn-sm">+ New</Link>
      </div>

      {repos.length === 0 ? (
        <div className="empty-state">
          <p>No repositories yet</p>
          <Link to="/new" style={{ fontSize: 14, textDecoration: 'underline' }}>Create one</Link>
        </div>
      ) : (
        repos.map(r => (
          <Link key={r.id} to={`/${r.user_id === 1 ? 'me' : r.id}/${r.name}`} className="repo-card">
            <div className="name">{r.name}</div>
            {r.description && <div className="desc">{r.description}</div>}
            <div className="meta">
              <span>{r.is_private ? 'Private' : 'Public'}</span>
              <span>{r.updated_at?.slice(0, 10)}</span>
            </div>
          </Link>
        ))
      )}
    </div>
  )
}
