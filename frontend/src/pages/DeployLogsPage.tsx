import { useEffect, useState } from 'react'
import { useParams, Link } from 'react-router-dom'
import { listDeploys, getRepo, type DeployLog, type Repo } from '../api'
import Spinner from '../components/Spinner'

export default function DeployLogsPage() {
  const { id } = useParams<{ id: string }>()
  const [repo, setRepo] = useState<Repo | null>(null)
  const [logs, setLogs] = useState<DeployLog[]>([])
  const [loading, setLoading] = useState(true)
  const [err, setErr] = useState('')

  useEffect(() => {
    if (!id) return
    const numId = parseInt(id)
    if (isNaN(numId)) { setErr('ID 無效'); setLoading(false); return }

    Promise.all([
      getRepo(numId),
      listDeploys(numId),
    ])
      .then(([r, l]) => {
        setRepo(r.repo)
        setLogs(l.deploy_logs)
      })
      .catch(e => setErr(e.message))
      .finally(() => setLoading(false))
  }, [id])

  if (!id) return <div className="error-box">缺少倉庫 ID</div>
  if (loading) return <Spinner />
  if (err) return <div className="error-box">{err}</div>
  if (!repo) return <div className="error-box">倉庫不存在</div>

  return (
    <div className="settings-page" style={{ maxWidth: 700 }}>
      <div className="breadcrumb">
        <Link to="/">~</Link> / <Link to={`/repo/${repo.id}`}>{repo.name}</Link> / <Link to={`/repo/${repo.id}/app`}>App</Link> / <strong>Deploy Logs</strong>
      </div>
      <h2>Deploy Logs</h2>

      {logs.length === 0 ? (
        <div className="empty-state">
          <p>No deployments yet.</p>
        </div>
      ) : (
        <div className="deploy-log-list">
          {logs.map(log => (
            <Link key={log.id} to={`/repo/${repo.id}/deploys/${log.id}`} className="deploy-log-entry">
              <span className={`deploy-status deploy-status-${log.status}`}>
                {log.status === 'running' && '⏳'}
                {log.status === 'success' && '✓'}
                {log.status === 'failed' && '✗'}
              </span>
              <span className="deploy-status-label">{log.status}</span>
              <span className="deploy-time">{log.started_at}</span>
              <span className="deploy-arrow">→</span>
            </Link>
          ))}
        </div>
      )}
    </div>
  )
}
