import { useEffect, useState } from 'react'
import { useParams, Link } from 'react-router-dom'
import Spinner from '../components/Spinner'
import { getOrg, listOrgMembers, addOrgMember, removeOrgMember, isLoggedIn } from '../api'
import type { Organization, OrgMember } from '../api'

export default function OrgMembers() {
  const { name } = useParams<{ name: string }>()
  const [org, setOrg] = useState<Organization | null>(null)
  const [members, setMembers] = useState<OrgMember[]>([])
  const [loading, setLoading] = useState(true)
  const [err, setErr] = useState('')
  const [addName, setAddName] = useState('')
  const [adding, setAdding] = useState(false)

  const load = () => {
    if (!name) return
    setLoading(true)
    Promise.all([getOrg(name), listOrgMembers(name)])
      .then(([orgRes, membersRes]) => {
        setOrg(orgRes.org)
        setMembers(membersRes.members)
      })
      .catch(e => setErr(e.message))
      .finally(() => setLoading(false))
  }

  useEffect(load, [name])

  const handleAdd = async () => {
    if (!name || !addName.trim()) return
    setAdding(true)
    setErr('')
    try {
      await addOrgMember(name, addName.trim())
      setAddName('')
      load()
    } catch (e: any) {
      setErr(e.message)
    } finally {
      setAdding(false)
    }
  }

  const handleRemove = async (userId: number) => {
    if (!name) return
    if (!confirm('確定移除成員？')) return
    try {
      await removeOrgMember(name, userId)
      load()
    } catch (e: any) {
      setErr(e.message)
    }
  }

  if (loading) return <Spinner />
  if (err) return <div className="error-box">{err}</div>
  if (!org) return <div className="error-box">組織不存在</div>
  if (!isLoggedIn()) return <div className="error-box">需要登入</div>

  return (
    <div className="page">
      <div className="breadcrumb">
        <Link to="/orgs">組織</Link> / <Link to={`/org/${org.name}`}>{org.display_name || org.name}</Link> / <strong>成員</strong>
      </div>

      <h2 style={{ marginTop: 16 }}>成員管理</h2>

      <div className="form" style={{ marginTop: 16 }}>
        {err && <div className="error-box">{err}</div>}
        <div style={{ display: 'flex', gap: 8, alignItems: 'flex-end' }}>
          <label style={{ flex: 1 }}>
            加入成員（輸入使用者名稱）
            <input value={addName} onChange={e => setAddName(e.target.value)} placeholder="username" />
          </label>
          <button className="btn" onClick={handleAdd} disabled={adding} style={{ marginBottom: 4 }}>
            {adding ? '加入中...' : '加入'}
          </button>
        </div>
      </div>

      <div className="repo-list" style={{ marginTop: 16 }}>
        {members.map(m => (
          <div key={m.id} className="repo-item" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <div className="repo-info">
              <Link to={`/u/${m.username}`} style={{ color: '#7c7c7c' }}>{m.username}</Link>
            </div>
            <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
              <span className="tag">{m.role}</span>
              <button className="btn-sm" onClick={() => handleRemove(m.user_id)}>移除</button>
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}
