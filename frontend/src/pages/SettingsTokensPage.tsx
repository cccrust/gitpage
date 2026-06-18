import { useEffect, useState, type FormEvent } from 'react'
import { listTokens, createToken, deleteToken } from '../api'
import type { AccessToken } from '../api'
import Spinner from '../components/Spinner'

export default function SettingsTokensPage() {
  const [tokens, setTokens] = useState<AccessToken[]>([])
  const [loading, setLoading] = useState(true)
  const [err, setErr] = useState('')
  const [name, setName] = useState('')
  const [creating, setCreating] = useState(false)
  const [rawToken, setRawToken] = useState('')

  const load = async () => {
    setLoading(true)
    setErr('')
    try {
      const d = await listTokens()
      setTokens(d.tokens)
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : '載入失敗')
    }
    setLoading(false)
  }

  useEffect(() => { load() }, [])

  const handleCreate = async (e: FormEvent) => {
    e.preventDefault()
    if (!name.trim()) return
    setCreating(true)
    setErr('')
    setRawToken('')
    try {
      const d = await createToken(name.trim())
      setRawToken(d.raw_token)
      setName('')
      await load()
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : '建立失敗')
    }
    setCreating(false)
  }

  const handleDelete = async (id: number) => {
    if (!confirm('確定刪除此 Token？')) return
    try {
      await deleteToken(id)
      await load()
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : '刪除失敗')
    }
  }

  if (loading) return <Spinner />

  return (
    <div className="settings-page" style={{ maxWidth: 600 }}>
      <h2>Access Tokens</h2>

      {rawToken && (
        <div style={{ background: '#fff3cd', padding: 12, borderRadius: 6, marginBottom: 16, fontSize: 13 }}>
          <strong>Token 已建立（只顯示一次）：</strong>
          <code style={{ display: 'block', marginTop: 6, wordBreak: 'break-all' }}>{rawToken}</code>
        </div>
      )}

      <form onSubmit={handleCreate}>
        <label>Token 名稱</label>
        <div style={{ display: 'flex', gap: 8 }}>
          <input type="text" value={name} onChange={e => setName(e.target.value)}
            placeholder="例如：my-laptop" style={{ flex: 1 }} />
          <button type="submit" disabled={creating || !name.trim()}>
            {creating ? '建立中...' : '建立'}
          </button>
        </div>
      </form>

      {err && <p className="msg-err">{err}</p>}

      <hr />
      <h3>已有 Token</h3>
      {tokens.length === 0 && <p style={{ fontSize: 13, color: '#666' }}>尚無任何 Token</p>}
      {tokens.map(t => (
        <div key={t.id} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '8px 0', borderBottom: '1px solid #eee' }}>
          <div>
            <strong>{t.name}</strong>
            <span style={{ fontSize: 12, color: '#888', marginLeft: 8 }}>{t.token_prefix}...</span>
            <span style={{ fontSize: 12, color: '#888', marginLeft: 8 }}>{t.scopes}</span>
          </div>
          <button className="btn-sm danger" onClick={() => handleDelete(t.id)}>刪除</button>
        </div>
      ))}
    </div>
  )
}
