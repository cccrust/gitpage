import { useEffect, useState } from 'react'
import { Link, useParams } from 'react-router-dom'
import Spinner from '../components/Spinner'
import { getOrg, listOrgRepos, isLoggedIn } from '../api'
import type { Organization, Repo } from '../api'

export default function OrgDetail() {
  const { name } = useParams<{ name: string }>()
  const [org, setOrg] = useState<Organization | null>(null)
  const [repos, setRepos] = useState<Repo[]>([])
  const [loading, setLoading] = useState(true)
  const [err, setErr] = useState('')

  useEffect(() => {
    if (!name) return
    setLoading(true)
    Promise.all([getOrg(name), listOrgRepos(name)])
      .then(([orgRes, reposRes]) => {
        setOrg(orgRes.org)
        setRepos(reposRes.repos)
      })
      .catch(e => setErr(e.message))
      .finally(() => setLoading(false))
  }, [name])

  if (loading) return <Spinner />
  if (err) return <div className="error-box">{err}</div>
  if (!org) return <div className="error-box">組織不存在</div>

  return (
    <div className="page">
      <div className="breadcrumb">
        <Link to="/orgs">組織</Link> / <strong>{org.display_name || org.name}</strong>
      </div>

      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginTop: 16 }}>
        <div>
          <h2>{org.display_name}</h2>
          {org.description && <p style={{ color: '#7c7c7c' }}>{org.description}</p>}
        </div>
        {isLoggedIn() && (
          <div className="actions">
            <Link to={`/org/${org.name}/members`} className="btn-sm">成員</Link>
            <Link to={`/org/${org.name}/settings`} className="btn-sm">設定</Link>
          </div>
        )}
      </div>

      <h3 style={{ marginTop: 24 }}>倉庫</h3>
      {repos.length === 0 ? (
        <div className="empty-state">
          <p>此組織還沒有倉庫</p>
        </div>
      ) : (
        <div className="repo-list">
          {repos.map(r => (
            <Link key={r.id} to={`/repo/${r.id}`} className="repo-item">
              <div className="repo-info">
                <strong>{r.name}</strong>
                <span className="desc">{r.description}</span>
              </div>
              <span className="tag">{r.is_private ? 'Private' : 'Public'}</span>
            </Link>
          ))}
        </div>
      )}
    </div>
  )
}
