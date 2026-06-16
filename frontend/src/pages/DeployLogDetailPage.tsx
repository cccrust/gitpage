import { useEffect, useState } from 'react'
import { useParams, Link } from 'react-router-dom'
import { getDeployLog, getRepo, type DeployLog, type Repo } from '../api'
import Spinner from '../components/Spinner'

export default function DeployLogDetailPage() {
  const { id, deployId } = useParams<{ id: string; deployId: string }>()
  const [repo, setRepo] = useState<Repo | null>(null)
  const [log, setLog] = useState<DeployLog | null>(null)
  const [loading, setLoading] = useState(true)
  const [err, setErr] = useState('')

  useEffect(() => {
    if (!id || !deployId) return
    const numId = parseInt(id)
    const numDeployId = parseInt(deployId)
    if (isNaN(numId) || isNaN(numDeployId)) {
      setErr('ID 無效')
      setLoading(false)
      return
    }

    Promise.all([
      getRepo(numId),
      getDeployLog(numId, numDeployId),
    ])
      .then(([r, l]) => {
        setRepo(r.repo)
        setLog(l.deploy_log)
      })
      .catch(e => setErr(e.message))
      .finally(() => setLoading(false))
  }, [id, deployId])

  if (!id) return <div className="error-box">缺少倉庫 ID</div>
  if (loading) return <Spinner />
  if (err) return <div className="error-box">{err}</div>
  if (!repo || !log) return <div className="error-box">找不到</div>

  const statusClass = `deploy-status-${log.status}`

  return (
    <div className="settings-page" style={{ maxWidth: 700 }}>
      <div className="breadcrumb">
        <Link to="/">~</Link> / <Link to={`/repo/${repo.id}`}>{repo.name}</Link> / <Link to={`/repo/${repo.id}/app`}>App</Link> / <Link to={`/repo/${repo.id}/deploys`}>Deploy Logs</Link> / <strong>#{log.id}</strong>
      </div>

      <div className="deploy-log-detail-header">
        <h2>
          <span className={`deploy-status ${statusClass}`}>
            {log.status === 'running' && '⏳'}
            {log.status === 'success' && '✓'}
            {log.status === 'failed' && '✗'}
          </span>
          {' '}Deploy #{log.id}
        </h2>
        <div className="deploy-meta">
          <span>Status: <strong className={statusClass}>{log.status}</strong></span>
          <span>Started: {log.started_at}</span>
          {log.finished_at && <span>Finished: {log.finished_at}</span>}
        </div>
      </div>

      <div className="deploy-log-output">
        <pre>{log.log_output || '(無輸出)'}</pre>
      </div>
    </div>
  )
}
