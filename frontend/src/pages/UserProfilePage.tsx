import { useEffect, useState } from 'react'
import { useParams, Link } from 'react-router-dom'
import { me, isLoggedIn } from '../api'
import type { User, Repo } from '../api'
import Spinner from '../components/Spinner'

export default function UserProfilePage() {
  const { username } = useParams<{ username: string }>()
  const [profile, setProfile] = useState<User | null>(null)
  const [repos, setRepos] = useState<Repo[]>([])
  const [loading, setLoading] = useState(true)
  const [err, setErr] = useState('')

  useEffect(() => {
    if (!username) return
    setLoading(true)
    fetch(`/api/users/${username}/profile`)
      .then(r => r.json())
      .then(d => {
        setProfile(d.user)
        setRepos(d.repos)
      })
      .catch(e => setErr(e.message))
      .finally(() => setLoading(false))
  }, [username])

  if (loading) return <Spinner />
  if (err) return <div className="error-box">{err}</div>
  if (!profile) return <div className="error-box">使用者不存在</div>

  return (
    <div className="profile-page">
      <div className="profile-header">
        <div className="avatar">{profile.username[0].toUpperCase()}</div>
        <h2>{profile.username}</h2>
        {profile.bio && <p className="bio">{profile.bio}</p>}
        <p style={{ fontSize: 12, color: '#7c7c7c' }}>
          Joined {profile.created_at?.slice(0, 10)}
        </p>
      </div>

      <h3 style={{ fontSize: 14, fontWeight: 600, margin: '16px 0 8px' }}>Repositories</h3>
      {repos.length === 0 ? (
        <div className="empty-state"><p>No public repositories</p></div>
      ) : (
        repos.map(r => (
          <Link key={r.id} to={`/repo/${r.id}`} className="repo-card">
            <div className="name">{r.name}</div>
            {r.description && <div className="desc">{r.description}</div>}
            <div className="meta">
              <span>Public</span>
              <span>{r.updated_at?.slice(0, 10)}</span>
            </div>
          </Link>
        ))
      )}
    </div>
  )
}
