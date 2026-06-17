import { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import Spinner from '../components/Spinner'
import { listMyOrgs } from '../api'
import type { OrganizationWithRole } from '../api'

export default function OrgList() {
  const [orgs, setOrgs] = useState<OrganizationWithRole[]>([])
  const [loading, setLoading] = useState(true)
  const [err, setErr] = useState('')

  useEffect(() => {
    listMyOrgs()
      .then(r => setOrgs(r.orgs))
      .catch(e => setErr(e.message))
      .finally(() => setLoading(false))
  }, [])

  if (loading) return <Spinner />
  if (err) return <div className="error-box">{err}</div>

  return (
    <div className="page">
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <h2>我的組織</h2>
        <Link to="/orgs/new" className="btn-sm">建立組織</Link>
      </div>

      {orgs.length === 0 ? (
        <div className="empty-state" style={{ marginTop: 24 }}>
          <p>你還沒有加入任何組織</p>
        </div>
      ) : (
        <div className="repo-list" style={{ marginTop: 16 }}>
          {orgs.map(o => (
            <Link key={o.id} to={`/org/${o.name}`} className="repo-item">
              <div className="repo-info">
                <strong>{o.display_name}</strong>
                <span className="desc">{o.description}</span>
              </div>
              <span className="tag">{o.role}</span>
            </Link>
          ))}
        </div>
      )}
    </div>
  )
}
