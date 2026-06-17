import { useEffect, useState, type FormEvent } from 'react'
import { useNavigate } from 'react-router-dom'
import { createRepo, listMyOrgs } from '../api'
import type { OrganizationWithRole } from '../api'

export default function NewRepoPage() {
  const [name, setName] = useState('')
  const [desc, setDesc] = useState('')
  const [priv, setPriv] = useState(false)
  const [orgs, setOrgs] = useState<OrganizationWithRole[]>([])
  const [org, setOrg] = useState('')
  const [err, setErr] = useState('')
  const nav = useNavigate()

  useEffect(() => {
    listMyOrgs().then(r => setOrgs(r.orgs)).catch(() => {})
  }, [])

  const submit = async (e: FormEvent) => {
    e.preventDefault()
    setErr('')
    try {
      const res = await createRepo(name, desc || undefined, priv, org || undefined)
      nav(`/repo/${res.repo.id}`)
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : '建立倉庫失敗')
    }
  }

  return (
    <div className="new-repo-page">
      <h2>New Repository</h2>
      <form onSubmit={submit}>
        <input type="text" placeholder="Repository name" value={name} onChange={e => setName(e.target.value)} required />
        <input type="text" placeholder="Description (optional)" value={desc} onChange={e => setDesc(e.target.value)} />
        {orgs.length > 0 && (
          <select value={org} onChange={e => setOrg(e.target.value)}>
            <option value="">個人帳號</option>
            {orgs.map(o => (
              <option key={o.id} value={o.name}>{o.display_name}</option>
            ))}
          </select>
        )}
        <label className="checkbox">
          <input type="checkbox" checked={priv} onChange={e => setPriv(e.target.checked)} />
          Private repository
        </label>
        {err && <p className="msg-err">{err}</p>}
        <button className="btn" type="submit">Create</button>
      </form>
    </div>
  )
}
