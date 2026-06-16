import { useEffect, useState, FormEvent } from 'react'
import { useParams, Link } from 'react-router-dom'
import { getRepo, getPagesConfig, updatePagesConfig, deployPages, type Repo, type PagesConfig } from '../api'
import Spinner from '../components/Spinner'

export default function PagesSettingsPage() {
  const { id } = useParams<{ id: string }>()
  const [repo, setRepo] = useState<Repo | null>(null)
  const [username, setUsername] = useState('')
  const [cfg, setCfg] = useState<PagesConfig | null>(null)
  const [branch, setBranch] = useState('main')
  const [sourceDir, setSourceDir] = useState('/')
  const [customDomain, setCustomDomain] = useState('')
  const [enabled, setEnabled] = useState(false)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [deploying, setDeploying] = useState(false)
  const [msg, setMsg] = useState('')
  const [err, setErr] = useState('')

  useEffect(() => {
    if (!id) return
    const numId = parseInt(id)
    if (isNaN(numId)) { setErr('ID 無效'); setLoading(false); return }

    setLoading(true)
    Promise.all([
      getRepo(numId),
      getPagesConfig(numId),
    ])
      .then(([repoRes, pagesRes]) => {
        setRepo(repoRes.repo)
        setUsername(repoRes.username)
        const c = pagesRes.pages_config
        setCfg(c)
        if (c) {
          setBranch(c.branch)
          setSourceDir(c.source_dir)
          setCustomDomain(c.custom_domain)
          setEnabled(c.enabled)
        } else {
          setBranch(repoRes.repo.default_branch || 'main')
        }
      })
      .catch(e => setErr(e.message))
      .finally(() => setLoading(false))
  }, [id])

  const submit = async (e: FormEvent) => {
    e.preventDefault()
    if (!id) return
    setSaving(true)
    setMsg('')
    setErr('')
    try {
      const res = await updatePagesConfig(parseInt(id), {
        branch, source_dir: sourceDir, custom_domain: customDomain, enabled,
      })
      if (res.deploy_error) {
        setMsg(`Saved, but deploy failed: ${res.deploy_error}`)
      } else {
        setMsg('Saved' + (enabled ? ' and deployed' : ''))
      }
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : '儲存失敗')
    }
    setSaving(false)
  }

  const handleDeploy = async () => {
    if (!id) return
    setDeploying(true)
    setMsg('')
    setErr('')
    try {
      const res = await deployPages(parseInt(id))
      setMsg(`Deployed to ${res.pages_dir}`)
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : '部署失敗')
    }
    setDeploying(false)
  }

  if (loading) return <Spinner />
  if (err && !repo) return <div className="error-box">{err}</div>
  if (!repo) return <div className="error-box">倉庫不存在</div>

  return (
    <div className="settings-page">
      <div className="breadcrumb">
        <Link to="/">~</Link> / <Link to={`/repo/${repo.id}`}>{repo.name}</Link> / Pages
      </div>
      <h2>Pages</h2>
      <p className="hint">Publish static files from your repository as a website.</p>

      <form onSubmit={submit}>
        <label>Branch</label>
        <input type="text" value={branch} onChange={e => setBranch(e.target.value)} required />

        <label>Source directory</label>
        <input type="text" value={sourceDir} onChange={e => setSourceDir(e.target.value)} placeholder="/" required />

        <label>Custom domain (optional)</label>
        <input type="text" value={customDomain} onChange={e => setCustomDomain(e.target.value)} placeholder="example.com" />

        <label className="checkbox">
          <input type="checkbox" checked={enabled} onChange={e => setEnabled(e.target.checked)} />
          Enabled
        </label>

        {msg && <p className="msg-ok">{msg}</p>}
        {err && <p className="msg-err">{err}</p>}

        <button className="btn" type="submit" disabled={saving}>
          {saving ? 'Saving...' : 'Save'}
        </button>
      </form>

      {enabled && (
        <div className="section">
          <h3>Deploy</h3>
          <p className="hint">
            Site URL: <a href={`/pages/${username}/${repo.name}/`} target="_blank" rel="noopener" style={{ textDecoration: 'underline' }}>
              /pages/{username}/{repo.name}/
            </a>
          </p>
          <button className="btn" onClick={handleDeploy} disabled={deploying}>
            {deploying ? 'Deploying...' : 'Redeploy'}
          </button>
        </div>
      )}
    </div>
  )
}
