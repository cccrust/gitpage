import { useEffect, useState } from 'react'
import { useParams } from 'react-router-dom'
import { listBranchProtections, createBranchProtection, deleteBranchProtection, isLoggedIn } from '../api'
import type { BranchProtection } from '../api'
import Spinner from '../components/Spinner'

export default function SettingsBranches() {
  const { id } = useParams()
  const [protections, setProtections] = useState<BranchProtection[]>([])
  const [pattern, setPattern] = useState('')
  const [requirePr, setRequirePr] = useState(true)
  const [requireApprovals, setRequireApprovals] = useState(1)
  const [dismissStale, setDismissStale] = useState(true)
  const [loading, setLoading] = useState(true)
  const [msg, setMsg] = useState('')

  const load = async () => {
    if (!id) return
    try {
      const d = await listBranchProtections(Number(id))
      setProtections(d.branch_protections)
    } catch { /* ignore */ }
    setLoading(false)
  }

  useEffect(() => { load() }, [id])

  const doCreate = async () => {
    if (!pattern.trim() || !id) return
    setMsg('')
    try {
      await createBranchProtection(Number(id), pattern.trim(), requirePr, requireApprovals, dismissStale)
      setPattern('')
      load()
    } catch (e: unknown) {
      setMsg(e instanceof Error ? e.message : '建立失敗')
    }
  }

  const doDelete = async (protectionId: number) => {
    if (!id) return
    try {
      await deleteBranchProtection(Number(id), protectionId)
      load()
    } catch (e: unknown) {
      setMsg(e instanceof Error ? e.message : '刪除失敗')
    }
  }

  if (!isLoggedIn()) return <div className="error-box">請先登入</div>
  if (loading) return <Spinner />

  return (
    <div style={{ maxWidth: 600 }}>
      <h2>分支保護規則</h2>

      {msg && <p style={{ color: '#d32f2f', fontSize: 13 }}>{msg}</p>}

      <section style={{ marginBottom: 24, display: 'flex', flexDirection: 'column', gap: 8 }}>
        <div style={{ display: 'flex', gap: 8 }}>
          <input value={pattern} onChange={e => setPattern(e.target.value)}
            placeholder="分支模式 (如 main, release/*)" style={{ flex: 1 }} />
          <button onClick={doCreate}>新增</button>
        </div>
        <label style={{ fontSize: 12 }}><input type="checkbox" checked={requirePr} onChange={e => setRequirePr(e.target.checked)} /> 需要 PR</label>
        <label style={{ fontSize: 12 }}><input type="checkbox" checked={dismissStale} onChange={e => setDismissStale(e.target.checked)} /> 過時審查自動撤銷</label>
        <label style={{ fontSize: 12 }}>最少審查人數：<input type="number" min={0} max={10} value={requireApprovals} onChange={e => setRequireApprovals(Number(e.target.value))} style={{ width: 60 }} /></label>
      </section>

      <table style={{ width: '100%', borderCollapse: 'collapse' }}>
        <thead>
          <tr style={{ textAlign: 'left', fontSize: 12, fontWeight: 600 }}>
            <th style={{ padding: '4px 8px' }}>分支模式</th>
            <th style={{ padding: '4px 8px' }}>需要 PR</th>
            <th style={{ padding: '4px 8px' }}>審查人數</th>
            <th style={{ padding: '4px 8px' }}>撤銷過時</th>
            <th style={{ padding: '4px 8px' }}>操作</th>
          </tr>
        </thead>
        <tbody>
          {protections.map(p => (
            <tr key={p.id} style={{ borderTop: '1px solid #eee', fontSize: 13 }}>
              <td style={{ padding: '8px' }}><code>{p.pattern}</code></td>
              <td style={{ padding: '8px' }}>{p.require_pr ? '✓' : '✗'}</td>
              <td style={{ padding: '8px' }}>{p.require_approvals}</td>
              <td style={{ padding: '8px' }}>{p.dismiss_stale_reviews ? '✓' : '✗'}</td>
              <td style={{ padding: '8px' }}>
                <button onClick={() => doDelete(p.id)} style={{ color: '#d32f2f' }}>刪除</button>
              </td>
            </tr>
          ))}
          {protections.length === 0 && (
            <tr><td colSpan={5} style={{ padding: 16, textAlign: 'center', color: '#999' }}>尚無分支保護規則</td></tr>
          )}
        </tbody>
      </table>
    </div>
  )
}
