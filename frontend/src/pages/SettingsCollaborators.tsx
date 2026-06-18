import { useEffect, useState } from 'react'
import { useParams } from 'react-router-dom'
import { listCollaborators, addCollaborator, removeCollaborator, isLoggedIn } from '../api'
import type { RepoCollaborator } from '../api'
import Spinner from '../components/Spinner'

export default function SettingsCollaborators() {
  const { id } = useParams()
  const [collabs, setCollabs] = useState<RepoCollaborator[]>([])
  const [username, setUsername] = useState('')
  const [permission, setPermission] = useState('write')
  const [loading, setLoading] = useState(true)
  const [msg, setMsg] = useState('')

  const load = async () => {
    if (!id) return
    try {
      const d = await listCollaborators(Number(id))
      setCollabs(d.collaborators)
    } catch { /* ignore */ }
    setLoading(false)
  }

  useEffect(() => { load() }, [id])

  const doAdd = async () => {
    if (!username.trim() || !id) return
    setMsg('')
    try {
      await addCollaborator(Number(id), username.trim(), permission)
      setUsername('')
      load()
    } catch (e: unknown) {
      setMsg(e instanceof Error ? e.message : '新增失敗')
    }
  }

  const doRemove = async (userId: number) => {
    if (!id) return
    try {
      await removeCollaborator(Number(id), userId)
      load()
    } catch (e: unknown) {
      setMsg(e instanceof Error ? e.message : '移除失敗')
    }
  }

  if (!isLoggedIn()) return <div className="error-box">請先登入</div>
  if (loading) return <Spinner />

  return (
    <div style={{ maxWidth: 600 }}>
      <h2>協作者管理</h2>

      {msg && <p style={{ color: '#d32f2f', fontSize: 13 }}>{msg}</p>}

      <section style={{ marginBottom: 24, display: 'flex', gap: 8, alignItems: 'center' }}>
        <input value={username} onChange={e => setUsername(e.target.value)}
          placeholder="使用者名稱" style={{ flex: 1 }} />
        <select value={permission} onChange={e => setPermission(e.target.value)}>
          <option value="read">唯讀</option>
          <option value="write">寫入</option>
          <option value="admin">管理</option>
        </select>
        <button onClick={doAdd}>新增</button>
      </section>

      <table style={{ width: '100%', borderCollapse: 'collapse' }}>
        <thead>
          <tr style={{ textAlign: 'left', fontSize: 12, fontWeight: 600 }}>
            <th style={{ padding: '4px 8px' }}>使用者</th>
            <th style={{ padding: '4px 8px' }}>權限</th>
            <th style={{ padding: '4px 8px' }}>操作</th>
          </tr>
        </thead>
        <tbody>
          {collabs.map(c => (
            <tr key={c.user_id} style={{ borderTop: '1px solid #eee', fontSize: 13 }}>
              <td style={{ padding: '8px' }}>{c.username}</td>
              <td style={{ padding: '8px' }}>{c.permission}</td>
              <td style={{ padding: '8px' }}>
                <button onClick={() => doRemove(c.user_id)} style={{ color: '#d32f2f' }}>移除</button>
              </td>
            </tr>
          ))}
          {collabs.length === 0 && (
            <tr><td colSpan={3} style={{ padding: 16, textAlign: 'center', color: '#999' }}>尚無協作者</td></tr>
          )}
        </tbody>
      </table>
    </div>
  )
}
