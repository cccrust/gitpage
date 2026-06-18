import { useEffect, useState } from 'react'
import { listTokens, createToken, deleteToken, isLoggedIn } from '../api'
import type { AccessToken } from '../api'
import Spinner from '../components/Spinner'

export default function SettingsTokens() {
  const [tokens, setTokens] = useState<AccessToken[]>([])
  const [name, setName] = useState('')
  const [newToken, setNewToken] = useState('')
  const [loading, setLoading] = useState(true)
  const [msg, setMsg] = useState('')

  const load = async () => {
    try {
      const d = await listTokens()
      setTokens(d.tokens)
    } catch { /* ignore */ }
    setLoading(false)
  }

  useEffect(() => { load() }, [])

  const doCreate = async () => {
    if (!name.trim()) return
    setMsg('')
    try {
      const d = await createToken(name.trim())
      setNewToken(d.raw_token)
      setName('')
      load()
    } catch (e: unknown) {
      setMsg(e instanceof Error ? e.message : '建立失敗')
    }
  }

  const doDelete = async (id: number) => {
    try {
      await deleteToken(id)
      load()
    } catch (e: unknown) {
      setMsg(e instanceof Error ? e.message : '刪除失敗')
    }
  }

  if (!isLoggedIn()) return <div className="error-box">請先登入</div>
  if (loading) return <Spinner />

  return (
    <div style={{ maxWidth: 600 }}>
      <h2>Personal Access Tokens</h2>

      {newToken && (
        <div className="error-box" style={{ background: '#e8f5e9', borderColor: '#4caf50' }}>
          <strong>Token 已建立，只顯示一次：</strong>
          <code style={{ display: 'block', padding: 8, marginTop: 8, background: '#222', color: '#0f0', borderRadius: 4, wordBreak: 'break-all' }}>
            {newToken}
          </code>
          <button onClick={() => setNewToken('')} style={{ marginTop: 8 }}>關閉</button>
        </div>
      )}

      {msg && <p style={{ color: '#d32f2f', fontSize: 13 }}>{msg}</p>}

      <section style={{ marginBottom: 24, display: 'flex', gap: 8 }}>
        <input value={name} onChange={e => setName(e.target.value)}
          placeholder="Token 名稱" style={{ flex: 1 }} />
        <button onClick={doCreate}>建立</button>
      </section>

      <table style={{ width: '100%', borderCollapse: 'collapse' }}>
        <thead>
          <tr style={{ textAlign: 'left', fontSize: 12, fontWeight: 600 }}>
            <th style={{ padding: '4px 8px' }}>名稱</th>
            <th style={{ padding: '4px 8px' }}>前綴</th>
            <th style={{ padding: '4px 8px' }}>權限</th>
            <th style={{ padding: '4px 8px' }}>建立時間</th>
            <th style={{ padding: '4px 8px' }}>操作</th>
          </tr>
        </thead>
        <tbody>
          {tokens.map(t => (
            <tr key={t.id} style={{ borderTop: '1px solid #eee', fontSize: 13 }}>
              <td style={{ padding: '8px' }}>{t.name}</td>
              <td style={{ padding: '8px' }}><code>{t.token_prefix}...</code></td>
              <td style={{ padding: '8px' }}>{t.scopes}</td>
              <td style={{ padding: '8px' }}>{t.created_at.slice(0, 10)}</td>
              <td style={{ padding: '8px' }}>
                <button onClick={() => doDelete(t.id)} style={{ color: '#d32f2f' }}>刪除</button>
              </td>
            </tr>
          ))}
          {tokens.length === 0 && (
            <tr><td colSpan={5} style={{ padding: 16, textAlign: 'center', color: '#999' }}>尚無 Token</td></tr>
          )}
        </tbody>
      </table>
    </div>
  )
}
