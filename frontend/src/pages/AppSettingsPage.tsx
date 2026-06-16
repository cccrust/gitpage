import { useEffect, useState, type FormEvent } from 'react'
import { useParams, Link } from 'react-router-dom'
import { getRepo, getAppsConfig, updateAppsConfig, deployApps, type Repo, type AppStatusResponse } from '../api'

export default function AppSettingsPage() {
  const { id } = useParams<{ id: string }>()
  const [repo, setRepo] = useState<Repo | null>(null)
  const [status, setStatus] = useState<AppStatusResponse | null>(null)
  const [branch, setBranch] = useState('main')
  const [sourceDir, setSourceDir] = useState('/')
  const [buildCommand, setBuildCommand] = useState('')
  const [startCommand, setStartCommand] = useState('')
  const [envVars, setEnvVars] = useState('{}')
  const [enabled, setEnabled] = useState(false)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [deploying, setDeploying] = useState(false)
  const [msg, setMsg] = useState('')
  const [err, setErr] = useState('')

  useEffect(() => {
    if (!id) return
    const numId = parseInt(id)
    if (isNaN(numId)) { setErr('Invalid ID'); setLoading(false); return }

    setLoading(true)
    Promise.all([
      getRepo(numId),
      getAppsConfig(numId),
    ])
      .then(([repoRes, appRes]) => {
        setRepo(repoRes.repo)
        setStatus(appRes)
        const c = appRes.apps_config
        if (c) {
          setBranch(c.branch)
          setSourceDir(c.source_dir)
          setBuildCommand(c.build_command)
          setStartCommand(c.start_command)
          setEnvVars(c.env_vars)
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
      const res = await updateAppsConfig(parseInt(id), {
        branch, source_dir: sourceDir,
        build_command: buildCommand, start_command: startCommand,
        env_vars: envVars, enabled,
      })
      if (res.deploy_error) {
        setMsg(`Saved, but deploy failed: ${res.deploy_error}`)
      } else {
        setMsg('Saved' + (enabled ? ' and deployed' : ''))
      }
      // Refresh status
      const appRes = await getAppsConfig(parseInt(id))
      setStatus(appRes)
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : 'Save failed')
    }
    setSaving(false)
  }

  const handleDeploy = async () => {
    if (!id) return
    setDeploying(true)
    setMsg('')
    setErr('')
    try {
      const res = await deployApps(parseInt(id))
      setMsg(`Deployed on port ${res.port}`)
      const appRes = await getAppsConfig(parseInt(id))
      setStatus(appRes)
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : 'Deploy failed')
    }
    setDeploying(false)
  }

  if (loading) return <div className="loading">Loading...</div>
  if (err && !repo) return <div className="error-box">{err}</div>
  if (!repo) return <div className="error-box">Repository not found</div>

  return (
    <div className="settings-page">
      <div className="breadcrumb">
        <Link to="/">~</Link> / <Link to={`/repo/${repo.id}`}>{repo.name}</Link> / App
      </div>
      <h2>App</h2>
      <p className="hint">Deploy a server-side application from your repository.</p>

      {status?.status && (
        <div className="section">
          <p style={{ color: status.status === 'running' ? '#16a34a' : status.status === 'deploying' ? '#d97706' : '#dc2626' }}>
            Status: <strong>{status.status}</strong>
            {status.port ? ` (port ${status.port})` : ''}
          </p>
        </div>
      )}

      <form onSubmit={submit}>
        <label>Branch</label>
        <input type="text" value={branch} onChange={e => setBranch(e.target.value)} required />

        <label>Source directory</label>
        <input type="text" value={sourceDir} onChange={e => setSourceDir(e.target.value)} placeholder="/" required />

        <label>Build command (leave empty for auto-detect)</label>
        <input type="text" value={buildCommand} onChange={e => setBuildCommand(e.target.value)} placeholder="npm install" />

        <label>Start command (leave empty for auto-detect)</label>
        <input type="text" value={startCommand} onChange={e => setStartCommand(e.target.value)} placeholder="npm start" />

        <label>Environment variables (JSON)</label>
        <textarea value={envVars} onChange={e => setEnvVars(e.target.value)} rows={3} placeholder='{"KEY": "value"}' />

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

      {enabled && status?.url && (
        <div className="section">
          <h3>Deploy</h3>
          <p className="hint">
            App URL: <a href={status.url} target="_blank" rel="noopener" style={{ textDecoration: 'underline' }}>
              {status.url}/
            </a>
          </p>
          <div style={{ display: 'flex', gap: 8, marginTop: 8 }}>
            <button className="btn" onClick={handleDeploy} disabled={deploying}>
              {deploying ? 'Deploying...' : 'Redeploy'}
            </button>
            <Link to={`/repo/${repo.id}/deploys`} className="btn-sm">Deploy Logs</Link>
          </div>
        </div>
      )}
    </div>
  )
}
