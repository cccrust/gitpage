import { useEffect, useState } from 'react'
import { useParams } from 'react-router-dom'
import { listSecrets, createSecret, deleteSecret, isLoggedIn } from '../api'
import type { RepoSecret } from '../api'
import Spinner from '../components/Spinner'

export default function SettingsSecrets() {
  const { id } = useParams()
  const [secrets, setSecrets] = useState<RepoSecret[]>([])
  const [name, setName] = useState('')
  const [value, setValue] = useState('')
  const [loading, setLoading] = useState(true)
  const [msg, setMsg] = useState('')

  const load = async () => {
    if (!id) return
    try {
      const d = await listSecrets(Number(id))
      setSecrets(d.secrets)
    } catch { /* ignore */ }
    setLoading(false)
  }

  useEffect(() => { load() }, [id])

  const doCreate = async () => {
    if (!name.trim() || !value.trim() || !id) return
    setMsg('')
    try {
      await createSecret(Number(id), name.trim(), value)
      setName('')
      setValue('')
      load()
    } catch (e: unknown) {
      setMsg(e instanceof Error ? e.message : '建立失敗')
    }
  }

  const doDelete = async (secretId: number) => {
    if (!id) return
    try {
      await deleteSecret(Number(id), secretId)
      load()
    } catch (e: unknown) {
      setMsg(e instanceof Error ? e.message : '刪除失敗')
    }
  }

  if (!isLoggedIn()) return <div className="error-box">請先登入</div>
  if (loading) return <Spinner />

  return (
    <div style={{ maxWidth: 600 }}>
      <h2>Secrets</h2>

      {msg && <p style={{ color: '#d32f2f', fontSize: 13 }}>{msg}</p>}

      <section style={{ marginBottom: 24 }}>
        <div style={{ display: 'flex', gap: 8, marginBottom: 8 }}>
          <input value={name} onChange={e => setName(e.target.value)}
            placeholder="名稱 (如 DEPLOY_KEY)" style={{ flex: 1 }} />
        </div>
        <div style={{ display: 'flex', gap: 8 }}>
          <input value={value} onChange={e => setValue(e.target.value)}
            placeholder="值" style={{ flex: 1 }} type="password" />
          <button onClick={doCreate}>新增</button>
        </div>
      </section>

      <table style={{ width: '100%', borderCollapse: 'collapse' }}>
        <thead>
          <tr style={{ textAlign: 'left', fontSize: 12, fontWeight: 600 }}>
            <th style={{ padding: '4px 8px' }}>名稱</th>
            <th style={{ padding: '4px 8px' }}>建立時間</th>
            <th style={{ padding: '4px 8px' }}>操作</th>
          </tr>
        </thead>
        <tbody>
          {secrets.map(s => (
            <tr key={s.id} style={{ borderTop: '1px solid #eee', fontSize: 13 }}>
              <td style={{ padding: '8px' }}><code>{s.name}</code></td>
              <td style={{ padding: '8px' }}>{s.created_at.slice(0, 10)}</td>
              <td style={{ padding: '8px' }}>
                <button onClick={() => doDelete(s.id)} style={{ color: '#d32f2f' }}>刪除</button>
              </td>
            </tr>
          ))}
          {secrets.length === 0 && (
            <tr><td colSpan={3} style={{ padding: 16, textAlign: 'center', color: '#999' }}>尚無 Secret</td></tr>
          )}
        </tbody>
      </table>
    </div>
  )
}
